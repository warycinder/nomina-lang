# nomina

A small text format for describing random name generators, plus a
command-line tool that validates a `.nomina` file and can reformat it into
a canonical layout.

## the problem

Most "random fantasy name generator" code I've seen is a pile of hardcoded
arrays and string concatenation buried in whatever script needed a name
that day. It's not reusable across projects and it's not something a
non-programmer could edit. I wanted a tiny, readable format instead:
something a writer or game designer could open in a text editor, tweak,
and trust that mistakes (a typo'd reference, a rule defined twice) get
caught with a precise pointer to the exact character that's wrong, not a
panic or a silently wrong result.

This repository is the front end for that: a lexer, a recursive-descent
parser, a validator, and a pretty printer. It does not generate names yet
(see Status below) — right now it's the part that makes sure a `.nomina`
file is well-formed before anything tries to use it.

## the format

A `.nomina` file is a list of rules. Each rule has a name and a pattern:

```
# a small grammar for elven first names
name = first " " last;

first = "Anna" | "Beorn" | "Coral" | "Dessa";
last  = "Vayle" | "Thorn" | "Marrow";
```

- `#` starts a comment that runs to the end of the line.
- `"..."` is a literal piece of text.
- a bare identifier refers to another rule.
- writing things next to each other (`first " " last`) concatenates them.
- `|` separates alternatives; one is picked at random.
- `?` after something makes it optional.
- `(...)` groups a pattern, mostly useful before `?` or inside `|`.
- every file needs exactly one rule named `name` — that's the entry point.

A rule combining grouping and optional pieces:

```
name = onset nucleus (coda)? (onset nucleus coda)?;

onset   = "b" | "br" | "k" | "kr" | "d" | "th";
nucleus = "a" | "e" | "i" | "o" | "u";
coda    = "n" | "r" | "th" | "s";
```

## usage

```
cargo run -- check names.nomina
cargo run -- fmt names.nomina
```

`check` parses and validates the file, printing nothing but a one-line
summary on success:

```
names.nomina: grammar is valid (3 rules)
```

`fmt` prints the canonical, reformatted version of the file to stdout, so
it can be piped to a new file or compared against the original with
`diff`.

## error messages

This is the part I care most about getting right. Every error points at
the exact line and column, with the offending source line and a caret
underline, in the same spirit as `rustc`'s diagnostics:

```
$ cargo run -- check names.nomina
error: undefined rule `tribe`
 --> names.nomina:4:28
  |
4 | last = "Vayle" | "Thorn" | tribe;
  | ^^^^^ not found in this file
1 error(s) found
```

Duplicate rule names point back at the first definition:

```
error: rule `last` is defined more than once (first defined at line 4)
 --> names.nomina:9:1
  |
9 | last = "Grey" | "Stone";
  | ^^^^
```

All validation errors in a file are collected and printed together
instead of stopping at the first one.

## pretty printing

`fmt` normalizes spacing and, for rules whose alternatives don't fit on
one line, breaks them out with one alternative per line:

```
title =
    "the Bold"
    | "the Wise"
    | "the Grey"
    | "the Undying"
    | "the Wanderer"
    | "of the North"
    ;
```

Comments are not preserved by `fmt` yet — see Status.

## status

Implemented: lexer, parser, duplicate/undefined-rule validation, and the
pretty printer described above.

Not implemented yet:

- actually generating a random name from a validated grammar
- preserving comments when pretty-printing
- weighted alternatives (`"common":5 | "rare":1`)
- detecting infinite recursion between rules
- string escapes and non-ASCII literal support beyond plain Unicode text

## building

Standard library only, no dependencies:

```
cargo build
```

## license

MIT, see LICENSE.
