#[cfg(feature = "test")]
mod test {
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
#[cfg(feature = "test")]
use test::*;

use {
    serde::{Deserialize, Serialize},
    serde_json::{from_reader, to_vec, to_writer, Error as SjError},
    std::{
        env::{var, VarError},
        fs::File,
        io::{BufReader, Error as IoError},
    },
    websocket::{
        dataframe::{DataFrame, Opcode},
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
#[derive(Deserialize, Serialize, Debug)]
struct Account {
    user: String,
    pass: String,
}
impl Account {
    fn new(user: String, pass: String) -> Self {
        Account { user, pass }
    }
    fn as_json(&self) -> Result<DataFrame, Error> {
        Ok(DataFrame::new(true, Opcode::Binary, to_vec(self)?))
    }
}

//  Database struct
#[derive(Deserialize, Serialize)]
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

    #[cfg(feature = "test")]
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
    Error,

    Total,
    Backup,
    Restore,

    Add(String, String),
    Find(String, String),

    #[cfg(feature = "test")]
    Random(Option<usize>),
}

fn main() {
    let mut data = Database::new();

    let server = Server::bind("127.0.0.1:80").unwrap();
    println!("Listening on {}", server.local_addr().unwrap());

    server.for_each(move |upgrade| {
        if let Ok(request) = upgrade {
            if !request.protocols().contains(&"rust-websocket".to_string()) {
                request.reject().unwrap();
                return;
            }

            let client = request.use_protocol("rust-websocket").accept().unwrap();
            let ip = client.peer_addr().unwrap();

            println!("Connection from {}", ip);

            let (mut receiver, mut sender) = client.split().unwrap();

            for message in receiver.incoming_messages() {
                if let Ok(message) = message {
                    match message {
                        Text(msg) => {
                            let mut cmd: Vec<String> = Vec::new();
                            for s in msg.splitn(3, " ") {
                                cmd.push(s.to_string())
                            }

                            //  Using `Command` variants because of `mpsc`
                            let command = match cmd.len() {
                                1 => match cmd[0].as_str() {
                                    "exit" | "quit" | "close" => break,
                                    "total" => Command::Total,
                                    "backup" => Command::Backup,
                                    "restore" => Command::Restore,
                                    _ => Command::Error,
                                },
                                2 => match cmd[0].as_str() {
                                    #[cfg(feature = "test")]
                                    "random" => Command::Random(if let Ok(n) = cmd[1].parse() {
                                        Some(n)
                                    } else {
                                        None
                                    }),
                                    _ => Command::Error,
                                },
                                3 => match cmd[0].as_str() {
                                    "add" => {
                                        Command::Add(cmd[1].clone(), cmd[2..].join(" ").clone())
                                    }
                                    "find" => {
                                        Command::Find(cmd[1].clone(), cmd[2..].join(" ").clone())
                                    }
                                    _ => Command::Error,
                                },
                                _ => Command::Error,
                            };

                            match command {
                                Command::Add(user, pass) => {
                                    match data.add(&user, &pass) {
                                        Ok(_) => {
                                            if let Err(error) = sender.send_dataframe(
                                                &DataFrame::new(true, Opcode::Text, vec![1]),
                                            ) {
                                                println!("ERROR  =>  {:?}", error)
                                            }
                                            if let Err(error) = sender.send_message(&Text(
                                                "Successfully Registered".to_string(),
                                            )) {
                                                println!("ERROR  =>  {:?}", error)
                                            }
                                        }
                                        Err(n) => {
                                            if let Err(error) =
                                                sender.send_message(&Text(ERRORS[n].to_string()))
                                            {
                                                println!("ERROR  =>  {:?}", error)
                                            }
                                        }
                                    }
                                }

                                Command::Find(user, pass) => match data.find(&user, &pass) {
                                    Ok(option) => match option {
                                        Some(account) => {
                                            if let Err(error) =
                                                sender.send_dataframe(&account.as_json().unwrap())
                                            {
                                                println!("ERROR  =>  {:?}", error)
                                            }
                                        }
                                        None => {
                                            if let Err(error) = sender.send_message(&Text(
                                                "Account not found".to_string(),
                                            )) {
                                                println!("ERROR  =>  {:?}", error)
                                            }
                                        }
                                    },
                                    Err(n) => println!("{}", ERRORS[n]),
                                },

                                Command::Total => {
                                    let mut total = 0;
                                    for v in data.0.iter() {
                                        total += v.len()
                                    }
                                    println!("{}", total)
                                }

                                Command::Backup => match data.backup() {
                                    Ok(_) => println!("Successfully backed up."),
                                    Err(e) => println!("Backup Error: {:?}.", e),
                                },

                                Command::Restore => match data.restore() {
                                    Ok(_) => println!("Successfully restored"),
                                    Err(e) => println!("Restore Error: {:?}.", e),
                                },

                                #[cfg(feature = "test")]
                                Command::Random(parsed) => match parsed {
                                    Some(n) => {
                                        data.generate_accounts(n);
                                        println!("Successfully generated {} random accounts.", n)
                                    }
                                    None => {
                                        println!("Failed to parse.")
                                    }
                                },
                                _ => (),
                            }
                            if let Err(error) = sender.send_message(&Text("close".to_string())) {
                                println!("ERROR  =>  {:?}", error)
                            }
                            //sender.shutdown_all();
                        }
                        Close(_) => {
                            if let Err(error) = sender.send_message(&Close(None)) {
                                println!("ERROR  =>  {:?}", error)
                            }
                            println!("Client {} disconnected", ip);
                            return;
                        }
                        _ => (),
                    }
                }
            }
        }
    })
}