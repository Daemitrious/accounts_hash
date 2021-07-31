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
    serde_json::{from_reader, from_slice, to_vec, to_writer, Error as JsonError},
    std::{
        env::{var, VarError},
        fs::File,
        io::{BufReader, Error as IoError},
        net::TcpStream,
    },
    websocket::{
        dataframe::{DataFrame, Opcode},
        server::{upgrade::sync::Buffer, InvalidConnection},
        sync::Server,
        OwnedMessage::{Binary, Close},
        WebSocketError,
    },
};

//  Lowest and highest possible hash values from `String::hash()`
const MIN: u32 = 555_819_297;
const MAX: u32 = 2_122_219_134;

//  A prime number of the value 1/1,000 times of `MIN`
const PRIME: u32 = 55579 /* 555_767 */;

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

//  Simple error handling
#[derive(Debug)]
enum Error {
    JsonError(JsonError),
    IoError(IoError),
    VarError(VarError),
    WsError(WebSocketError),
    RejectError((TcpStream, IoError)),
    InvalidConnection(InvalidConnection<TcpStream, Buffer>),
}
impl From<JsonError> for Error {
    fn from(error: JsonError) -> Self {
        Error::JsonError(error)
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
impl From<WebSocketError> for Error {
    fn from(error: WebSocketError) -> Self {
        Error::WsError(error)
    }
}
impl From<(TcpStream, IoError)> for Error {
    fn from(error: (TcpStream, IoError)) -> Self {
        Error::RejectError(error)
    }
}
impl From<InvalidConnection<TcpStream, Buffer>> for Error {
    fn from(error: InvalidConnection<TcpStream, Buffer>) -> Self {
        Error::InvalidConnection(error)
    }
}

//  Base account struct
#[derive(Deserialize, Serialize, Debug, Clone)]
struct Account {
    user: String,
    pass: String,
    id: usize,
}
impl Account {
    fn new(user: String, pass: String, id: usize) -> Self {
        Account { user, pass, id }
    }
    fn as_json(&self) -> Result<DataFrame, Error> {
        Ok(DataFrame::new(true, Opcode::Binary, to_vec(self)?))
    }
}

//  Database struct
#[derive(Deserialize, Serialize)]
struct Database(Vec<Vec<Account>>, usize);
impl Database {
    //  Build new database with preinitialized vectors
    fn new() -> Self {
        Self(
            (0..((MAX - MIN) / PRIME) + 1).map(|_| Vec::new()).collect(),
            0,
        )
    }

    //  Search method based on the hash value of the Account's `pass`
    fn find(&self, user: String, pass: String) -> Option<&Account> {
        let row = &self.0[user.hash()];
        if row.len() > 0 {
            for account in row.iter() {
                if user == account.user && pass == account.pass {
                    return Some(account);
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
                self.0[user.hash()].push(account.clone());
                self.1 += 1;
                Ok(account)
            }
        }
    }

    //  Backup the database
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
                if let Ok(_) = self.add(&randstr(4..15), &randstr(8..25)) {
                    break;
                }
            }
        }
    }
}

fn main() -> Result<(), Error> {
    //  WebSocket Protocol
    const PROTOCOL: &str = "rust-websocket";

    //  Commands
    const FIND: &str = "find";
    const ADD: &str = "add";

    //  Applicable with the contain method of a `request`
    let protocol = &PROTOCOL.to_string();

    //  If `Err` from successful request then reply with a Blob with size of 0
    let invalid = DataFrame::new(true, Opcode::Binary, vec![]);

    //  The entire database
    let mut data = Database::new();

    //  Begin WebSocket Server
    let server = Server::bind("127.0.0.1:80")?;
    println!("Listening to {:?}\n", server.local_addr()?);

    server.for_each(move |upgrade| {
        if let Err(error) = {
            || -> Result<(), Error> {
                let request = upgrade?;

                if request.protocols().contains(protocol) {
                    let client = request.use_protocol(PROTOCOL).accept()?;

                    let ip = client.peer_addr()?;

                    println!("Connection from {}", ip);

                    let (mut receiver, mut sender) = client.split()?;

                    for message in receiver.incoming_messages() {
                        let msg = message?;

                        match msg {
                            Binary(v) => {
                                println!("Received :: {:?}", v);

                                //  If Blob is sent, has to be in the form of a `Post`.
                                let args = from_slice::<Vec<String>>(&v)?;

                                if let Err(error) = Ok({
                                    match args.len() {
                                        3 => match args[0].as_str() {
                                            FIND => sender.send_dataframe(&match data
                                                .find(args[1].clone(), args[2].clone())
                                            {
                                                Some(account) => account.as_json()?,
                                                None => invalid.clone(),
                                            })?,

                                            ADD => sender.send_dataframe(&match data
                                                .add(args[1].clone(), args[2].clone())
                                            {
                                                Ok(account) => account.as_json()?,
                                                Err(_) => invalid.clone(),
                                            })?,

                                            _ => unreachable!(),
                                        },
                                        _ => unreachable!(),
                                    }
                                }) {
                                    sender.shutdown_all()?;
                                    return Err(error);
                                }
                            }
                            Close(_) => {
                                sender.send_message(&Close(None))?;
                                println!("Client {} disconnected", ip);
                            }
                            _ => unreachable!(),
                        }
                    }
                } else {
                    println!("{:?}", request.reject()?);
                }
                Ok(())
            }
        }() {
            println!("{:?}", error)
        };
    });
    Ok(())
}
