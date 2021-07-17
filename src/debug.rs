use {
    rand::Rng,
    std::{ops::Range, time::SystemTime},
};

const ASCII: [char; 94] = [
    '@', '`', '!', 'A', 'a', '"', 'B', 'b', '#', 'C', 'c', '$', 'D', 'd', '%', 'E', 'e', '&', 'F',
    'f', '\'', 'G', 'g', '(', 'H', 'h', ')', 'I', 'i', '*', 'J', 'j', '+', 'K', 'k', ',', 'L', 'l',
    '-', 'M', 'm', '.', 'N', 'n', '/', 'O', 'o', '0', 'P', 'p', '1', 'Q', 'q', '2', 'R', 'r', '3',
    'S', 's', '4', 'T', 't', '5', 'U', 'u', '6', 'V', 'v', '7', 'W', 'w', '8', 'X', 'x', '9', 'Y',
    'y', ':', 'Z', 'z', ';', '[', '{', '<', '\\', '|', '=', ']', '}', '>', '^', '~', '?', '_',
];

pub fn randint(range: Range<usize>) -> usize {
    rand::thread_rng().gen_range(range)
}

pub fn randstr(size: Range<usize>) -> String {
    let mut string = "".to_owned();
    for _ in 0..randint(size) {
        string.push_str(&ASCII[randint(0..93)].to_string())
    }
    string
}

//  const FIND: &str = "find";

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

    //  Iteration method
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

/*
    //  Search using a basic `for` loop
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
*/
