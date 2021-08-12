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
    native_tls::{Identity, TlsAcceptor},
    serde::{Deserialize, Serialize},
    serde_json::{from_reader, from_slice, to_vec, to_writer},
    std::{
        env::var,
        error::Error as Errorable,
        fmt::Debug,
        fs::File,
        io::{BufReader, Error as IoError, Read},
        net::{TcpListener, TcpStream},
        ops::{Index, IndexMut},
        sync::{Arc, Mutex},
        thread::spawn,
    },
    tungstenite::{server::accept, Message, Message::Binary},
};

//  Lowest and highest possible hash values from `String::hash()`
const MIN: u32 = 555_819_297;
const MAX: u32 = 2_122_219_134;

//  A prime number of the value one 10,000th of `MIN`
const PRIME: u32 = 55_579 /* 555_767 */;

//  Applies a `hash` function to `String` to conveniently grab the native endian integer value
trait Hashable {
    fn hash(&self) -> u32;
}
impl Hashable for String {
    fn hash(&self) -> u32 {
        let mut bytes: [u8; 4] = [0; 4];
        bytes.copy_from_slice(self[..4].as_bytes());
        (u32::from_ne_bytes(bytes) - MIN) / PRIME
    }
}

//  Simple error handling
enum Error {
    Text(String),
}

impl<T> From<T> for Error
where
    T: Errorable,
{
    fn from(error: T) -> Self {
        Self::Text(error.to_string())
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self::Text(s) = self;
        f.write_str(s)
    }
}

//  Base account struct
#[derive(Deserialize, Serialize, Debug, Clone)]
struct Account {
    email: Option<String>,
    user: String,
    pass: String,
    id: usize,
}
impl Account {
    fn new(user: String, pass: String, id: usize) -> Self {
        Account {
            email: None,
            user,
            pass,
            id,
        }
    }
    fn as_json(&self) -> Result<Vec<u8>, Error> {
        Ok(to_vec(self)?)
    }
}

//  Database struct
#[derive(Deserialize, Serialize)]
struct Database(Vec<Vec<Account>>, usize);

impl Index<u32> for Database {
    type Output = Vec<Account>;

    fn index(&self, index: u32) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl IndexMut<u32> for Database {
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

impl Database {
    //  Build new database with preinitialized vectors
    fn new() -> Self {
        Self(
            (0..((MAX - MIN) / PRIME) + 1).map(|_| Vec::new()).collect(),
            0,
        )
    }

    //  Search method based on the hash value of the Account's `pass`
    fn find(&self, user: String, pass: String) -> Option<Account> {
        let row = &self[user.hash()];
        if row.len() > 0 {
            for account in row.iter() {
                if user == account.user && pass == account.pass {
                    return Some(account.clone());
                }
            }
        }
        None
    }

    //  Push new specified account into vector of the hash value of `pass` if `available_username`
    fn add(&mut self, user: String, pass: String) -> Result<Account, ()> {
        match self.find(user.clone(), pass.clone()) {
            Some(_) => Err(()),
            None => {
                let account = Account::new(user.clone(), pass, self.1 + 1);
                self[user.hash()].push(account.clone());
                self.1 += 1;
                Ok(account)
            }
        }
    }

    //  Backup the database  |  Might spawn as a new thread
    fn _backup(&self) -> Result<(), Error> {
        Ok(to_writer(File::create(var("BACKUP_PATH")?)?, &self.0)?)
    }

    //  Set the database to the most recent backup
    fn _restore(&mut self) -> Result<(), Error> {
        self.0 = from_reader(BufReader::new(File::open(var("BACKUP_PATH")?)?))?;
        Ok(())
    }

    #[cfg(feature = "test")]
    //  Randomly generate and push `n` random accounts to the database
    fn generate_accounts(&mut self, amount: usize) {
        for _ in 0..amount {
            loop {
                if let Ok(_) = self.add(randstr(4..15), randstr(8..25)) {
                    break;
                }
            }
        }
    }
}

fn handle_client(
    stream: Result<TcpStream, IoError>,
    thread_acceptor: Arc<TlsAcceptor>,
    thread_data: Arc<Mutex<Database>>,
) -> Result<(), Error> {
    let mut websocket = accept(thread_acceptor.accept(stream?)?)?;

    if let Binary(v) = websocket.read_message()? {
        //  All data recieved must be in the form of a JSON parsed array
        let args = from_slice::<Vec<String>>(&v)?;

        match thread_data.lock() {
            Ok(mut data) => match args.len() {
                3 => match args[0].as_str() {
                    "find" => websocket.write_message(Message::binary(
                        match data.find(args[1].clone(), args[2].clone()) {
                            Some(account) => account.as_json()?,
                            None => vec![],
                        },
                    ))?,

                    "add" => websocket.write_message(Message::binary(
                        match data.add(args[1].clone(), args[2].clone()) {
                            Ok(account) => account.as_json()?,
                            Err(_) => vec![],
                        },
                    ))?,
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            },
            Err(error) => println!("{:?}", error),
        }
    }
    Ok(())
}

fn main() -> Result<(), Error> {
    let identity = Identity::from_pkcs12(
        &{
            let mut identity = vec![];
            File::open("identity.pfx")?.read_to_end(&mut identity)?;
            identity
        },
        "PASSWORD",
    )?;

    //  TLS Acceptor
    let acceptor = Arc::new(TlsAcceptor::new(identity)?);

    //  The entire database
    let data = Arc::new(Mutex::new(Database::new()));

    //  Begin running the server
    for stream in TcpListener::bind("192.168.4.30:100")?.incoming() {
        let thread_data = data.clone();
        let thread_acceptor = acceptor.clone();

        spawn(move || {
            if let Err(Error::Text(error)) = handle_client(stream, thread_acceptor, thread_data) {
                println!("{}", error)
            }
        });
    }
    Ok(())
}
