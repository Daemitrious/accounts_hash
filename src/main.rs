#[cfg(feature = "benchmark")]
mod benchmark {
    pub use std::time::{Duration, SystemTime};

    #[derive(Debug)]
    pub struct Benchmark<T> {
        pub content: T,
        pub elapsed: Duration,
    }
}

#[cfg(feature = "benchmark")]
use benchmark::*;

#[cfg(feature = "testing")]
mod testing {
    use {
        rand::{thread_rng, Rng},
        std::ops::Range,
    };

    const ASCII: [char; 94] = [
        '@', '`', '!', 'A', 'a', '"', 'B', 'b', '#', 'C', 'c', '$', 'D', 'd', '%', 'E', 'e', '&',
        'F', 'f', '\'', 'G', 'g', '(', 'H', 'h', ')', 'I', 'i', '*', 'J', 'j', '+', 'K', 'k', ',',
        'L', 'l', '-', 'M', 'm', '.', 'N', 'n', '/', 'O', 'o', '0', 'P', 'p', '1', 'Q', 'q', '2',
        'R', 'r', '3', 'S', 's', '4', 'T', 't', '5', 'U', 'u', '6', 'V', 'v', '7', 'W', 'w', '8',
        'X', 'x', '9', 'Y', 'y', ':', 'Z', 'z', ';', '[', '{', '<', '\\', '|', '=', ']', '}', '>',
        '^', '~', '?', '_',
    ];

    fn randint(range: Range<usize>) -> usize {
        thread_rng().gen_range(range)
    }

    pub fn randstr(size: Range<usize>) -> String {
        (0..randint(size))
            .map(|_| ASCII[randint(0..93)])
            .collect::<String>()
    }
}
#[cfg(feature = "testing")]
use testing::*;

use {
    serde::{Deserialize, Serialize},
    serde_json::{from_reader, /*  from_slice, to_vec,  */ to_writer, Error as SjError},
    std::{
        env::{var, VarError},
        fs::File,
        io::{/*  stdin,  */ BufReader, Error as IoError},
        sync::mpsc::channel,
        thread::spawn,
    },
    websocket::{
        sync::Server,
        OwnedMessage::{Close, Text},
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

//  Simple error handling
#[derive(Debug)]
enum Error {
    SjError(SjError),
    IoError(IoError),
    VarError(VarError),
}
impl From<SjError> for Error {
    fn from(error: SjError) -> Self {
        Error::SjError(error)
    }
}
impl From<IoError> for Error {
    fn from(error: IoError) -> Self {
        Error::IoError(error)
    }
}
impl From<VarError> for Error {
    fn from(error: VarError) -> Self {
        Error::VarError(error)
    }
}

//  Applies a `hash` function to `String` to conveniently grab the native endian integer value
trait Hashable {
    fn hash(&self) -> usize;
}
impl Hashable for String {
    fn hash(&self) -> usize {
        let mut bytes: [u8; 4] = [0; 4];
        bytes.copy_from_slice(self[..4].as_bytes());
        ((u32::from_ne_bytes(bytes) - MIN) / PRIME) as usize
    }
}
/*
//  Simple `stdin`
fn input() -> Result<String, IoError> {
    let mut line = String::new();
    stdin().read_line(&mut line)?;
    Ok(line)
}
*/
//  Determine if `user` and `pass` are legal and compatible strings
fn check(user: &str, pass: &str) -> Result<(), usize> {
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
#[derive(Deserialize, Serialize, Debug, Clone)]
struct Account {
    user: String,
    pass: String,
}
impl Account {
    fn new(user: String, pass: String) -> Self {
        Account { user, pass }
    }
}

//  Database struct
#[derive(Deserialize, Serialize, Debug, Clone)]
struct Database(Vec<Vec<Account>>);

impl Database {
    //  Build new database with preinitialized vectors
    fn new() -> Self {
        Self((0..((MAX - MIN) / PRIME) + 1).map(|_| Vec::new()).collect())
    }

    //  Push new specified account into vector of the hash value of `pass` if `available_username`
    fn add(&mut self, user: &str, pass: &str) -> Result<(), usize> {
        match self.available_username(&user, &pass) {
            Ok(account) => Ok(self.0[account.user.hash()].push(account)),
            Err(why) => Err(why),
        }
    }

    //  Search method based on the hash value of the Account's `pass`
    fn find(&self, user: &str, pass: &str) -> Result<Option<&Account>, usize> {
        check(user, pass)?;

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

    //  Determine if `user` and `pass` are legal strings and `user` doesn't aleady exist
    fn available_username(&self, user: &str, pass: &str) -> Result<Account, usize> {
        check(user, pass)?;

        for account in self.0[user.to_string().hash()].iter() {
            if account.user == user {
                return Err(0);
            }
        }
        Ok(Account::new(user.to_string(), pass.to_string()))
    }

    //  Backup the database
    fn backup(&self) -> Result<(), Error> {
        Ok(to_writer(File::create(var("BACKUP_PATH")?)?, &self.0)?)
    }

    //  Set the database to the most recent backup
    fn restore(&mut self) -> Result<(), Error> {
        self.0 = from_reader(BufReader::new(File::open(var("BACKUP_PATH")?)?))?;
        Ok(())
    }

    #[cfg(feature = "benchmark")]
    //  Search method based on a basic `for loop`
    fn normal(&self, user: &str, pass: &str) -> Result<Option<Benchmark<&Account>>, usize> {
        check(user, pass)?;

        let a = SystemTime::now();

        for x in self.0.iter() {
            for account in x.iter() {
                if account.user == user && account.pass == pass {
                    let b = SystemTime::now();

                    return Ok(Some(Benchmark {
                        content: account,
                        elapsed: b.duration_since(a).unwrap(),
                    }));
                }
            }
        }
        Ok(None)
    }

    #[cfg(feature = "benchmark")]
    //  Search method based on the hash value of the Account's `pass`
    fn find_bm(&self, user: &str, pass: &str) -> Benchmark<Result<Option<&Account>, usize>> {
        let a = SystemTime::now();
        let result = self.find(user, pass);
        let b = SystemTime::now();

        Benchmark {
            content: result,
            elapsed: b.duration_since(a).unwrap(),
        }
    }

    #[cfg(feature = "testing")]
    //  Randomly generate and push `n` random accounts to the database
    fn generate_accounts(&mut self, amount: usize) {
        for _ in 0..amount {
            loop {
                if let Ok(_) = self.add(&randstr(4..15), &randstr(8..25)) {
                    break;
                }
            }
        }
    }
}

enum Command {
    Total,
    Backup,
    Restore,

    Add(String, String),
    Find(String, String),

    #[cfg(feature = "benchmark")]
    Normal(String, String),
    #[cfg(feature = "testing")]
    Random(Option<usize>),

    Error,
}

fn main() {
    let (tx, rx) = channel::<Command>();

    let mut data = Database::new();

    drop(data.add("Username", "Password"));

    let server = Server::bind("127.0.1.1:3000").unwrap();
    println!("Listening on {}", server.local_addr().unwrap());

    for request in server.filter_map(Result::ok) {
        let tx = tx.clone();

        // Spawn a new thread for each connection.
        spawn(move || {
            if !request.protocols().contains(&"rust-websocket".to_string()) {
                request.reject().unwrap();
                return;
            }

            let client = request.use_protocol("rust-websocket").accept().unwrap();
            let ip = client.peer_addr().unwrap();

            println!("Connection from {}", ip);

            let (mut receiver, mut sender) = client.split().unwrap();

            for message in receiver.incoming_messages() {
                let message = message.unwrap();

                match message {
                    Text(msg) => {
                        let mut cmd: Vec<String> = Vec::new();
                        for s in msg.splitn(3, " ") {
                            cmd.push(s.to_string())
                        }
                        println!("---\n{:?}\n---\n{} {}", cmd, cmd[1], cmd[2..].join(" "));

                        let command = match cmd.len() {
                            1 => match cmd[0].as_str() {
                                "exit" | "quit" | "close" => break,
                                "total" => Command::Total,
                                "backup" => Command::Backup,
                                "restore" => Command::Restore,
                                _ => Command::Error,
                            },
                            2 => match cmd[0].as_str() {
                                #[cfg(feature = "testing")]
                                "random" => Command::Random(if let Ok(n) = cmd[1].parse() {
                                    Some(n)
                                } else {
                                    None
                                }),
                                _ => Command::Error,
                            },
                            3 => match cmd[0].as_str() {
                                "add" => Command::Add(cmd[1].clone(), cmd[2..].join(" ").clone()),
                                "find" => Command::Find(cmd[1].clone(), cmd[2..].join(" ").clone()),
                                #[cfg(feature = "benchmark")]
                                "normal" => {
                                    Command::Normal(cmd[1].clone(), cmd[2..].join(" ").clone())
                                }
                                _ => Command::Error,
                            },
                            _ => Command::Error,
                        };
                        drop(tx.send(command));
                    }
                    Close(_) => {
                        sender.send_message(&Close(None)).unwrap();
                        println!("Client {} disconnected", ip);
                        return;
                    }
                    _ => (),
                }
            }
        });

        match rx.recv() {
            Ok(cmd) => match cmd {
                Command::Add(user, pass) => {
                    println!(
                        "{}",
                        if let Err(n) = data.add(&user, &pass) {
                            ERRORS[n]
                        } else {
                            "0"
                        }
                    )
                }

                #[cfg(not(feature = "benchmark"))]
                Command::Find(user, pass) => match data.find(&user, &pass) {
                    Ok(account) => println!("{:?}", account),
                    Err(n) => println!("{}", ERRORS[n]),
                },

                #[cfg(feature = "benchmark")]
                Command::Find(user, pass) => {
                    let Benchmark { content, elapsed } = data.find_bm(&user, &pass);
                    match content {
                        Ok(account) => println!("{:?} => {:?}", elapsed, account),
                        Err(n) => println!("{}", ERRORS[n]),
                    }
                }

                Command::Total => {
                    let mut total = 0;
                    for v in data.0.iter() {
                        total += v.len()
                    }
                    println!("{}", total);
                }

                Command::Backup => match data.backup() {
                    Ok(_) => println!("0"),
                    Err(e) => println!("{:?}", e),
                },

                Command::Restore => match data.restore() {
                    Ok(_) => println!("0"),
                    Err(e) => println!("{:?}", e),
                },

                #[cfg(feature = "testing")]
                Command::Random(parsed) => {
                    println!(
                        "{}",
                        if let Some(n) = parsed {
                            data.generate_accounts(n);
                            "0"
                        } else {
                            "1"
                        }
                    )
                }

                #[cfg(feature = "benchmark")]
                Command::Normal(user, pass) => match data.normal(&user, &pass) {
                    Ok(option) => match option {
                        Some(benchmark) => {
                            let Benchmark { content, elapsed } = benchmark;
                            println!("{:?} => {:?}", elapsed, content)
                        }
                        None => println!("None"),
                    },
                    Err(n) => println!("{}", ERRORS[n]),
                },
                Command::Error => (),
            },
            Err(error) => {
                println!("{:?}", error)
            }
        }
    }
}
