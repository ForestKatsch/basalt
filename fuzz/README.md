# Fuzz Testing

Requires cargo-fuzz: `cargo install cargo-fuzz`

## Running

```bash
# Fuzz the full compile pipeline
cargo fuzz run fuzz_compile

# Fuzz just the lexer
cargo fuzz run fuzz_lex

# Run for a specific time
cargo fuzz run fuzz_compile -- -max_total_time=60
```
