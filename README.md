# accounts_hash
This project stores basic `user` and `pass` accounts into a database tuple struct `Vec<Vec<Account>>`.

# Benchmarking
Based on a database holding just over 10 million accounts, on average, it takes just less than **100**µs (*microseconds*) to find the a specific account.
