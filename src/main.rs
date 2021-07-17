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
const PRIME: u32 = 555_767;

//  Base Commands
const TOTAL: &str = "total";
const EXIT: &str = "exit";
const QUIT: &str = "quit";

//  Data Commands
const ADD: &str = "add";
const FIND: &str = "find";

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

//  Path to backup file
const _BACKUP_PATH: &str = "accounts.json";

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
#[derive(Deserialize, Serialize)]
struct Database(Vec<Vec<Account>>);

impl Database {
    //  Determine if `user` and `pass` are legal strings and `user` doesn't aleady exist
    async fn available_username(&self, user: &str, pass: &str) -> Result<Account, usize> {
        if let Err(why) = check(user, pass).await {
            Err(why)
        } else {
            for account in self.0[user.to_string().hash()].iter() {
                if account.user == user {
                    return Err(0);
                }
            }
            Ok(Account::new(user.to_string(), pass.to_string()).await)
        }
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
    //  Backup the database
    async fn _backup(&self) -> Result<Result<(), sj_Error>, io_Error> {
        Ok(to_writer(File::create("accounts.json")?, &self.0))
    }
    //  Set the database to the most recent backup
    async fn _restore(&mut self) -> Result<(), io_Error> {
        self.0 = from_reader(BufReader::new(File::open(_BACKUP_PATH)?))?;
        Ok(())
    }
    //  Build new database with preinitialized vectors
    fn new() -> Self {
        Self((0..((MAX - MIN) / PRIME) + 1).map(|_| Vec::new()).collect())
    }
}

#[tokio::main]
async fn main() {
    let mut data = Database::new();

    //  My first attempt at a `repl`
    loop {
        if let Ok(string) = input().await {
            let string = string.trim_end();

            match string {
                TOTAL => {
                    let mut total = 0;
                    for v in data.0.iter() {
                        total += v.len()
                    }
                    println!("{}", total);
                    continue;
                }
                EXIT | QUIT => break,
                _ => (),
            }

            let cmd = string.split_ascii_whitespace().collect::<Vec<&str>>();

            if cmd.len() == 3 {
                match cmd[0] {
                    ADD => println!(
                        "{}",
                        match data.add(cmd[1], cmd[2]).await {
                            Ok(_) => "Added",
                            Err(n) => ERRORS[n],
                        }
                    ),
                    FIND => match data.find(cmd[1], cmd[2]).await {
                        Ok(option) => println!("FOUND :: {:?}", option),
                        Err(n) => println!("{}", ERRORS[n]),
                    },
                    _ => (),
                }
            }
        }
    }
}
