# Third-Party Reference Material

The projects below are pinned as specification and differential-test references.
They are not Cargo, Gerbil, build-time, link-time, or runtime dependencies, and
no source code from them is vendored into this repository.

## openCypher

- Repository: https://github.com/opencypher/openCypher
- Pinned revision: `677cbafabb8c3c5eed458fd3b1ec0daec8d67d23`
- Reviewed material: `grammar/openCypher.bnf` and `tck/features/`
- License: Apache-2.0
- Use: language-surface and conformance-oracle reference only

## SeleneDB

- Repository: https://github.com/jscott3201/selene-db
- Pinned revision: `74819dad96bc08549e104d5b48c96e382173014e`
- Reviewed material: `crates/selene-gql/src/parser/grammar.pest`
- License: Apache-2.0 OR MIT
- Use: parser-coverage comparison reference only

These pins authorize reference comparison only. The repository's Gerbil
declaration and generated native ABI remain the sole grammar authority.
