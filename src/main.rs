//  Debugging   ////////////
mod debug;

use debug::randstr;
use std::time::SystemTime;
////////////////////////////

use {
    serde::{
        ser::{SerializeStruct, SerializeTupleStruct},
        Serialize,
    },
    std::{convert::TryFrom, fs::File},
};

//  Used to determine size of main vector and downscale hash values generated from `Account::hash`
const MIN: u32 = 555_819_297;
const MAX: u32 = 2_122_219_134;

//  Greatest Prime number of the value 1,000 lesser than `MIN`
const PRIME: u32 = 555_767;

trait Hashable {
    fn hash(&self) -> usize;
}
impl Hashable for String {
    fn hash(&self) -> usize {
        ((u32::from_ne_bytes(TryFrom::try_from(self[..4].as_bytes()).unwrap()) - MIN) / PRIME)
            as usize
    }
}

#[derive(Debug)]
struct Account {
    user: String,
    pass: String,
}
impl Account {
    fn new(user: &str, pass: &str) -> Option<Self> {
        if user.is_ascii()
            && pass.is_ascii()
            && user.len() > 0
            && pass.len() > 7
            && user.len() < 15
            && pass.len() < 30
        {
            Some(Account {
                user: user.to_string(),
                pass: pass.to_string(),
            })
        } else {
            None
        }
    }
    fn random() -> Self {
        Account {
            user: randstr(1..15),
            pass: randstr(8..30),
        }
    }
}
impl Serialize for Account {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("Account", 2)?;
        s.serialize_field("user", &self.user)?;
        s.serialize_field("pass", &self.pass)?;
        s.end()
    }
}

struct Database(Vec<Vec<Account>>);

impl Database {
    fn add(&mut self, account: Account) -> Result<(), Account> {
        if account.pass.hash() == 4294967295 {
            return Err(account);
        }
        Ok(self.0[account.pass.hash()].push(account))
    }
    fn find(&self, user: String, pass: String) -> Option<&Account> {
        let row = &self.0[pass.hash()];
        if row.len() > 0 {
            for account in row.iter() {
                if user == account.user && pass == account.pass {
                    return Some(account);
                }
            }
            None
        } else {
            None
        }
    }
    fn new() -> Self {
        Self((0..((MAX - MIN) / PRIME) + 1).map(|_| Vec::new()).collect())
    }
    //  Make asyncronous at some point
    fn backup(&self) {
        println!(
            "{}",
            match serde_json::to_writer(File::create("accounts.json").unwrap(), &self.0) {
                Ok(_) => "Success",
                Err(_) => "Error",
            }
        )
    }
}
impl Serialize for Database {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut ts = serializer.serialize_tuple_struct("Database", 1)?;
        ts.serialize_field(&self.0)?;
        ts.end()
    }
}

fn main() {
    let mut data = Database::new();

    //  Generate a ton of random accounts
    for _ in 0..1_000_000 {
        if let Err(account) = data.add(Account::random()) {
            println!("{:?}", account)
        }
    }
    //  Add a specific account
    let test = Account::new("OwenIsGay", "SuhjinTheFuck69").unwrap();
    drop(data.add(Account::new("OwenIsGay", "SuhjinTheFuck69").unwrap()));

    //  Benchmark how long it takes to find that specific account
    let a = SystemTime::now();
    let option = data.find(test.user, test.pass);
    let b = SystemTime::now();

    //  Checks if the find was successful
    match option {
        Some(user) => println!("{:?}", user),
        _ => println!("None"),
    }

    //  How long it took to find the account
    println!("{:?}\n", b.duration_since(a));

    //  The total amount of accounts inside of the database
    println!("Total amount of accounts = {}", {
        let mut total = 0;
        for row in data.0.iter() {
            total += row.len()
        }
        total
    });
    data.backup()
}
