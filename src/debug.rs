use {rand::Rng, std::ops::Range};

//  TP
const ASCII: [char; 94] = [
    '@', '`', '!', 'A', 'a', '"', 'B', 'b', '#', 'C', 'c', '$', 'D', 'd', '%', 'E', 'e', '&', 'F',
    'f', '\'', 'G', 'g', '(', 'H', 'h', ')', 'I', 'i', '*', 'J', 'j', '+', 'K', 'k', ',', 'L', 'l',
    '-', 'M', 'm', '.', 'N', 'n', '/', 'O', 'o', '0', 'P', 'p', '1', 'Q', 'q', '2', 'R', 'r', '3',
    'S', 's', '4', 'T', 't', '5', 'U', 'u', '6', 'V', 'v', '7', 'W', 'w', '8', 'X', 'x', '9', 'Y',
    'y', ':', 'Z', 'z', ';', '[', '{', '<', '\\', '|', '=', ']', '}', '>', '^', '~', '?', '_',
];

//  TP
pub fn randint(range: Range<usize>) -> usize {
    rand::thread_rng().gen_range(range)
}
//  TP
pub fn randstr(size: Range<usize>) -> String {
    let mut string = "".to_owned();
    for _ in 0..randint(size) {
        string.push_str(&ASCII[randint(0..93)].to_string())
    }
    string
}
