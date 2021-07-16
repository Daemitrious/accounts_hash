//  Debugging   ////////////
//mod debug;

//use debug::randstr;
use std::time::SystemTime;
////////////////////////////

use {
    serde::{
        ser::{SerializeStruct, SerializeTupleStruct},
        Serialize,
    },
    serde_json::{to_writer, Error},
    std::{convert::TryFrom, fs::File, io::stdin},
    Reason::*,
};

//  Used to determine size of main vector and downscale hash values generated from `Account::hash`
const MIN: u32 = 555_819_297;
const MAX: u32 = 2_122_219_134;

//  Greatest Prime number of the value 1,000 lesser than `MIN`
const PRIME: u32 = 555_767;

//  Base Commands
const TOTAL: &str = "total";
const EXIT: &str = "exit";
const QUIT: &str = "quit";

//  Data Commands
const ADD: &str = "add";
const FIND: &str = "find";
const NORMAL: &str = "normal";

async fn input() -> Result<String, ()> {
    let mut line = String::new();

    match stdin().read_line(&mut line) {
        Ok(_) => Ok(line),
        Err(_) => Err(()),
    }
}

enum Reason {
    UserExists,

    UserNotAscii,
    UserLenLess,
    UserLenLong,

    PassNotAscii,
    PassLenLess,
    PassLenLong,
}

impl Reason {
    fn to_string(&self) -> String {
        match self {
            UserExists => "Username is already taken",

            UserNotAscii => "Username is not ASCII",
            UserLenLess => "Username is too short",
            UserLenLong => "Username is too long",

            PassNotAscii => "Password is not ASCII",
            PassLenLess => "Password is too short",
            PassLenLong => "Password is too long",
        }
        .to_string()
    }
}

async fn check(user: &str, pass: &str) -> Result<(), Reason> {
    if !user.is_ascii() {
        Err(UserNotAscii)
    } else if !pass.is_ascii() {
        Err(PassNotAscii)
    } else if !(user.len() > 3) {
        Err(UserLenLess)
    } else if !(pass.len() > 7) {
        Err(PassLenLess)
    } else if !(user.len() < 15) {
        Err(UserLenLong)
    } else if !(pass.len() < 25) {
        Err(PassLenLong)
    } else {
        Ok(())
    }
}

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
    async fn new(user: String, pass: String) -> Self {
        Account { user, pass }
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
    async fn available_username(&self, user: &str, pass: &str) -> Result<Account, Reason> {
        if let Err(why) = check(user, pass).await {
            Err(why)
        } else {
            for account in self.0[user.to_string().hash()].iter() {
                if account.user == user {
                    return Err(UserExists);
                }
            }
            Ok(Account::new(user.to_string(), pass.to_string()).await)
        }
    }

    async fn add(&mut self, user: &str, pass: &str) -> Result<(), Reason> {
        match self.available_username(&user, &pass).await {
            Ok(account) => Ok(self.0[account.user.hash()].push(account)),
            Err(why) => Err(why),
        }
    }
    //  Search method based on the hash value of the Account's `pass`
    async fn find(&self, user: &str, pass: &str) -> Result<Option<&Account>, Reason> {
        if let Err(why) = check(user, pass).await {
            Err(why)
        } else {
            let row = &self.0[user.to_string().hash()];
            if row.len() > 0 {
                for account in row.iter() {
                    if user == account.user && pass == account.pass {
                        return Ok(Some(account));
                    }
                }
                Ok(None)
            } else {
                Ok(None)
            }
        }
    }
    //  Basic for iteration
    async fn normal(&self, user: &str, pass: &str) -> Result<Option<&Account>, Reason> {
        if let Err(why) = check(user, pass).await {
            Err(why)
        } else {
            for x in self.0.iter() {
                for account in x.iter() {
                    if account.user == user && account.pass == pass {
                        return Ok(Some(account));
                    }
                }
            }
            Ok(None)
        }
    }
    fn new() -> Self {
        Self((0..((MAX - MIN) / PRIME) + 1).map(|_| Vec::new()).collect())
    }
    //  Make asyncronous at some point
    async fn _backup(&self) -> Result<(), Error> {
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

#[tokio::main]
async fn main() {
    let mut data = Database::new();
    /*
        //  The total amount of random accounts to be generated
        let amount = 1_000_000;

        println!("Generating {} accounts...", amount);

        for _ in 0..amount {
            loop {
                let (user, pass) = (&randstr(4..15), &randstr(8..25));
                match data.add(user, pass).await {
                    Err(why) => println!("{} => \"{}\"", why.to_string(), user),
                    _ => break,
                }
            }
        }
        //  The testing account's ("TA") username and password
        let (user, pass) = ("John", "EatMyWhale69");

        //  Add the TA to the filled database
        drop(data.add(user, pass).await);

        //  Hash method
        let a1 = SystemTime::now();
        let f1 = data.find(user, pass).await;
        let b1 = SystemTime::now();

        //  Basic for loop
        let a2 = SystemTime::now();
        let f2 = data.normal(user, pass).await;
        let b2 = SystemTime::now();

        let t1 = b1.duration_since(a1).unwrap();
        let t2 = b2.duration_since(a2).unwrap();

        //  Checks if the find was successful
        println!(
                "\nHash Method === {} === [{:?}]\nFor Loop === {} === [{:?}]\n\nHash over for loop :: {:.2}\nTotal amount of accounts :: {}\n",
                if let Some(_) = match f1 {
                    Ok(option) => option,
                    Err(why) => {
                        println!("{}", why.to_string());
                        None
                    }
                } { "Pass" } else { "Fail" },
                t1,
                if let Some(_) = match f2 {
                    Ok(option) => option,
                    Err(why) => {
                        println!("{}", why.to_string());
                        None
                    }
                } { "Pass" } else { "Fail" },
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
    */
    loop {
        if let Ok(string) = input().await {
            match string.trim_end() {
                TOTAL => {
                    let mut total = 0;
                    for v in data.0.iter() {
                        total += v.len()
                    }
                    println!("{}", total);
                    continue;
                }
                EXIT | QUIT => return,
                _ => (),
            }

            let cmd = string
                .trim_end()
                .split_ascii_whitespace()
                .collect::<Vec<&str>>();

            if cmd.len() == 3 {
                match cmd[0] {
                    ADD => println!(
                        "{}",
                        match data.add(cmd[1], cmd[2]).await {
                            Ok(_) => "Success".to_string(),
                            Err(why) => why.to_string(),
                        }
                    ),
                    FIND => println!("{:?}", {
                        let a = SystemTime::now();
                        let result = data.find(cmd[1], cmd[2]).await;
                        let b = SystemTime::now();
                        (
                            b.duration_since(a).unwrap(),
                            match result {
                                Ok(option) => {
                                    if let Some(account) = option {
                                        format!("{} {}", account.user, account.pass)
                                    } else {
                                        "None".to_string()
                                    }
                                }
                                Err(why) => why.to_string(),
                            },
                        )
                    }),
                    NORMAL => println!("{:?}", {
                        let a = SystemTime::now();
                        let result = data.normal(cmd[1], cmd[2]).await;
                        let b = SystemTime::now();
                        (
                            b.duration_since(a).unwrap(),
                            match result {
                                Ok(option) => {
                                    if let Some(account) = option {
                                        format!("{} {}", account.user, account.pass)
                                    } else {
                                        "None".to_string()
                                    }
                                }
                                Err(why) => why.to_string(),
                            },
                        )
                    }),
                    _ => (),
                }
            }
        }
    }
}
