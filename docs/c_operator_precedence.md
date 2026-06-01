# C Operator Precedence Reference

Source: ISO/IEC 9899:2011 (C11), §6.5 and Annex A.  
Used as reference for the Pratt parser binding-power table in [`src/parser/precedence.rs`](../src/parser/precedence.rs).

---

## Full C Precedence Table (highest → lowest)

| Level | Operators | Description | Associativity |
|-------|-----------|-------------|---------------|
| 15 | `()` `[]` `.` `->` `++` `--` (postfix) | Postfix / member access | Left → Right |
| 14 | `++` `--` (prefix) `+` `-` (unary) `!` `~` `*` (deref) `&` (addrof) `sizeof` `(type)` cast | Prefix unary / cast | Right → Left |
| 13 | `*` `/` `%` | Multiplicative | Left → Right |
| 12 | `+` `-` | Additive | Left → Right |
| 11 | `<<` `>>` | Bitwise shift | Left → Right |
| 10 | `<` `<=` `>` `>=` | Relational | Left → Right |
| 9  | `==` `!=` | Equality | Left → Right |
| 8  | `&` | Bitwise AND | Left → Right |
| 7  | `^` | Bitwise XOR | Left → Right |
| 6  | `\|` | Bitwise OR | Left → Right |
| 5  | `&&` | Logical AND | Left → Right |
| 4  | `\|\|` | Logical OR | Left → Right |
| 3  | `?:` | Ternary conditional | Right → Left |
| 2  | `=` `+=` `-=` `*=` `/=` `%=` `&=` `^=` `\|=` `<<=` `>>=` | Assignment | Right → Left |
| 1  | `,` | Comma | Left → Right |

---

## Mapping to Pratt binding powers

In a Pratt parser, **lbp** (left binding power) determines when the operator fires; **rbp** (right binding power) is passed as `min_bp` to the recursive right-side parse.

- **Left-associative**: `rbp = lbp + 1` — a same-precedence operator on the right does NOT re-associate.
- **Right-associative**: `rbp = lbp` — a same-precedence operator on the right DOES re-associate.

### Currently implemented operators

| Operator(s) | lbp | rbp | Associativity | C level |
|-------------|-----|-----|---------------|---------|
| `=` `+=` `-=` `*=` `/=` `%=` `&=` `^=` `\|=` `<<=` `>>=` | 1 | 1 | Right | 2 |
| `?:` | 2 | 2 | Right | 3 |
| `\|\|` | 4 | 5 | Left | 4 |
| `&&` | 6 | 7 | Left | 5 |
| `\|` | 8 | 9 | Left | 6 |
| `^` | 10 | 11 | Left | 7 |
| `&` | 12 | 13 | Left | 8 |
| `==` `!=` | 14 | 15 | Left | 9 |
| `<` `<=` `>` `>=` | 16 | 17 | Left | 10 |
| `<<` `>>` | 18 | 19 | Left | 11 |
| `+` `-` | 20 | 21 | Left | 12 |
| `*` `/` `%` | 22 | 23 | Left | 13 |
| prefix unary / cast | — | 30 | Right | 14 |
| postfix `++` `--` `[]` `()` `.` `->` | > 30 | — | Left | 15 |

### Operators not yet implemented

| Operator(s) | C level | Notes |
|-------------|---------|-------|
| `,` | 1 | Comma operator (below assignment) |
