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
    serde_json::{to_writer, Error},
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
    fn add(&mut self, account: Account) {
        self.0[account.pass.hash()].push(account)
    }
    //  Search method based on the hash value of the Account's `pass`
    fn find(&self, user: &str, pass: &str) -> Option<&Account> {
        let row = &self.0[pass.to_string().hash()];
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
    //  Basic for iteration
    fn normal(&self, user: &str, pass: &str) -> Option<&Account> {
        for x in self.0.iter() {
            for account in x.iter() {
                if account.user == user && account.pass == pass {
                    return Some(account);
                }
            }
        }
        None
    }
    fn new() -> Self {
        Self((0..((MAX - MIN) / PRIME) + 1).map(|_| Vec::new()).collect())
    }
    //  Make asyncronous at some point
    fn _backup(&self) -> Result<(), Error> {
        to_writer(File::create("accounts.json").unwrap(), &self.0)
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

    //  The total amount of random accounts to be generated
    let amount = 10_000_000;

    println!("Generating {} accounts...", amount);

    for _ in 0..amount {
        data.add(Account::random())
    }
    //  The testing account's ("TA") username and password
    let (user, pass) = ("John", "EatMyWhale69");

    //  Create the TA
    let test = Account::new(user, pass).unwrap();

    //  Add the TA to the filled database
    data.add(test);

    //  Hash method
    let a1 = SystemTime::now();
    let f1 = data.find(user, pass);
    let b1 = SystemTime::now();

    //  Basic for loop
    let a2 = SystemTime::now();
    let f2 = data.normal(user, pass);
    let b2 = SystemTime::now();

    let t1 = b1.duration_since(a1).unwrap();
    let t2 = b2.duration_since(a2).unwrap();

    //  Checks if the find was successful
    println!(
        "\nHash Method === {} === [{:?}]\nFor Loop === {} === [{:?}]\n\nHash over for loop :: {:.2}\nTotal amount of accounts :: {}\n",
        if let Some(_) = f1 { "Pass" } else { "Fail" },
        t1,
        if let Some(_) = f2 { "Pass" } else { "Fail" },
        t2,
        t2.as_secs_f64() / t1.as_secs_f64(),

        {
            let mut total = 0;
            for row in data.0.iter() {
                total += row.len()
            }
            total
        }
    );
}
