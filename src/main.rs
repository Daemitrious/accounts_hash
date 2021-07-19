#[cfg(feature = "benchmark")]
mod benchmark;
#[cfg(feature = "benchmark")]
use benchmark::*;

#[cfg(feature = "testing")]
mod testing;
#[cfg(feature = "testing")]
use testing::*;

use {
    serde::{Deserialize, Serialize},
    serde_json::{from_reader, to_writer, Error as sj_Error},
    std::{
        convert::TryFrom,
        fs::File,
        io::{stdin, BufReader, Error as io_Error},
    },
};

//  Lowest and highest possible hash values from `String::hash()`
const MIN: u32 = 555_819_297;
const MAX: u32 = 2_122_219_134;

//  A prime number of the value 1/1,000 times of `MIN`
const PRIME: u32 = 55579 /* 555_767 */;

//  Error Messages
const ERRORS: [&str; 7] = [
    "Username is already taken",
    "Username is not ASCII",
    "Username is too short",
    "Username is too long",
    "Password is not ASCII",
    "Password is too short",
    "Password is too long",
];

//  Applies a `hash` function to `String` to conveniently grab the native endian integer value
trait Hashable {
    fn hash(&self) -> usize;
}
impl Hashable for String {
    fn hash(&self) -> usize {
        ((u32::from_ne_bytes(TryFrom::try_from(self[..4].as_bytes()).unwrap()) - MIN) / PRIME)
            as usize
    }
}

//  Simple `stdin`
async fn input() -> Result<String, io_Error> {
    let mut line = String::new();
    stdin().read_line(&mut line)?;
    Ok(line)
}
//  Determine if `user` and `pass` are legal and compatible strings
async fn check(user: &str, pass: &str) -> Result<(), usize> {
    if !user.is_ascii() {
        Err(1)
    } else if !(user.len() > 3) {
        Err(2)
    } else if !(user.len() < 15) {
        Err(3)
    } else if !pass.is_ascii() {
        Err(4)
    } else if !(pass.len() > 7) {
        Err(5)
    } else if !(pass.len() < 25) {
        Err(6)
    } else {
        Ok(())
    }
}

//  Base account struct
#[derive(Deserialize, Serialize, Debug)]
struct Account {
    user: String,
    pass: String,
}
impl Account {
    async fn new(user: String, pass: String) -> Self {
        Account { user, pass }
    }
}

//  Database struct
#[derive(Deserialize, Serialize, Debug)]
struct Database(Vec<Vec<Account>>);

impl Database {
    //  Build new database with preinitialized vectors
    fn new() -> Self {
        Self((0..((MAX - MIN) / PRIME) + 1).map(|_| Vec::new()).collect())
    }

    //  Determine if `user` and `pass` are legal strings and `user` doesn't aleady exist
    async fn available_username(&self, user: &str, pass: &str) -> Result<Account, usize> {
        check(user, pass).await?;

        for account in self.0[user.to_string().hash()].iter() {
            if account.user == user {
                return Err(0);
            }
        }
        Ok(Account::new(user.to_string(), pass.to_string()).await)
    }

    //  Push new specified account into vector of the hash value of `pass` if `available_username`
    async fn add(&mut self, user: &str, pass: &str) -> Result<(), usize> {
        match self.available_username(&user, &pass).await {
            Ok(account) => Ok(self.0[account.user.hash()].push(account)),
            Err(why) => Err(why),
        }
    }

    //  Search method based on the hash value of the Account's `pass`
    async fn find(&self, user: &str, pass: &str) -> Result<Option<&Account>, usize> {
        check(user, pass).await?;

        let row = &self.0[user.to_string().hash()];
        if row.len() > 0 {
            for account in row.iter() {
                if user == account.user && pass == account.pass {
                    return Ok(Some(account));
                }
            }
        }
        Ok(None)
    }

    //  Backup the database
    async fn _backup(&self) -> Result<Result<(), sj_Error>, io_Error> {
        Ok(to_writer(File::create("accounts.json")?, &self.0))
    }

    //  Set the database to the most recent backup
    async fn _restore(&mut self) -> Result<(), io_Error> {
        self.0 = from_reader(BufReader::new(File::open("accounts.json")?))?;
        Ok(())
    }

    #[cfg(feature = "testing")]
    //  Randomly generate and push `n` random accounts to the database
    async fn _gen_accounts(&mut self, amount: usize) {
        for _ in 0..amount {
            loop {
                if let Ok(_) = self.add(&randstr(4..15), &randstr(8..25)).await {
                    break;
                }
            }
        }
    }

    #[cfg(feature = "benchmark")]
    //  Search method based on a basic `for loop`
    async fn _normal(&self, user: &str, pass: &str) -> Result<Option<&Account>, usize> {
        check(user, pass).await?;

        for x in self.0.iter() {
            for account in x.iter() {
                if account.user == user && account.pass == pass {
                    return Ok(Some(account));
                }
            }
        }
        Ok(None)
    }

    #[cfg(feature = "benchmark")]
    //  Search method based on the hash value of the Account's `pass`
    async fn _find(&self, user: &str, pass: &str) -> Benchmark<Result<Option<&Account>, usize>> {
        let a = SystemTime::now();
        let result = self.find(user, pass).await;
        let b = SystemTime::now();

        Benchmark {
            content: result,
            elapsed: b.duration_since(a).unwrap(),
        }
    }
}

#[tokio::main]
async fn main() {
    let mut data = Database::new();

    //  My attempt at a `repl`
    loop {
        if let Ok(string) = input().await {
            let cmd = string
                .trim_end()
                .split_ascii_whitespace()
                .collect::<Vec<&str>>();

            match cmd.len() {
                1 => match cmd[0] {
                    "exit" | "quit" => break,
                    "total" => {
                        let mut total = 0;
                        for v in data.0.iter() {
                            total += v.len()
                        }
                        println!("{}", total);
                    }
                    "backup" => println!(
                        "{}",
                        match data._backup().await {
                            Ok(_) => "0",
                            Err(_) => "1",
                        }
                    ),
                    "restore" => println!(
                        "{}",
                        match data._restore().await {
                            Ok(_) => "0",
                            Err(_) => "1",
                        }
                    ),
                    _ => (),
                },
                2 => match cmd[0] {
                    #[cfg(feature = "testing")]
                    "random" => println!(
                        "{}",
                        if let Ok(n) = cmd[1].parse() {
                            data._gen_accounts(n).await;
                            "0"
                        } else {
                            "1"
                        }
                    ),
                    _ => (),
                },
                3 => match cmd[0] {
                    "add" => {
                        println!(
                            "{}",
                            if let Err(n) = data.add(cmd[1], cmd[2]).await {
                                ERRORS[n]
                            } else {
                                "1"
                            }
                        )
                    }
                    "find" => match data.find(cmd[1], cmd[2]).await {
                        Ok(account) => println!("{:?}", account),
                        Err(n) => println!("{}", ERRORS[n]),
                    },

                    #[cfg(feature = "benchmark")]
                    "_find" => {
                        let Benchmark { content, elapsed } = data._find(cmd[1], cmd[2]).await;
                        match content {
                            Ok(account) => println!("{:?} => {:?}", elapsed, account),
                            Err(n) => println!("{}", ERRORS[n]),
                        }
                    }
                    _ => (),
                },
                _ => (),
            }
        }
    }
}
