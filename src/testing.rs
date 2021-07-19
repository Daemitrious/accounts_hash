use {
    rand::{thread_rng, Rng},
    std::ops::Range,
};

const ASCII: [char; 94] = [
    '@', '`', '!', 'A', 'a', '"', 'B', 'b', '#', 'C', 'c', '$', 'D', 'd', '%', 'E', 'e', '&', 'F',
    'f', '\'', 'G', 'g', '(', 'H', 'h', ')', 'I', 'i', '*', 'J', 'j', '+', 'K', 'k', ',', 'L', 'l',
    '-', 'M', 'm', '.', 'N', 'n', '/', 'O', 'o', '0', 'P', 'p', '1', 'Q', 'q', '2', 'R', 'r', '3',
    'S', 's', '4', 'T', 't', '5', 'U', 'u', '6', 'V', 'v', '7', 'W', 'w', '8', 'X', 'x', '9', 'Y',
    'y', ':', 'Z', 'z', ';', '[', '{', '<', '\\', '|', '=', ']', '}', '>', '^', '~', '?', '_',
];

fn randint(range: Range<usize>) -> usize {
    thread_rng().gen_range(range)
}

pub fn randstr(size: Range<usize>) -> String {
    (0..randint(size))
        .map(|_| ASCII[randint(0..93)])
        .collect::<String>()
}
