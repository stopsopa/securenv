I need rust setup which will do compilation of code from directory src/ and produce binaries in bin/ for mac arm64 locally. (bin/arm64)

(tell me also how to install rust tools on my mac btw)

Implement compile.sh (currently empty) - it should use the rust toolchain to validate and compile the code and produce binaries in bin/.

then having that create source code in src/ for program as follow

I need program which once executed like this

```sh
./bin/arm64 encrypt .env.tmp .env --mask ENCODEMASK --password "supersecretpassword"
```

that program will open file .env.tmp then it will read all it's env vars to te form of key value native rust data structure and keep it all of these in memory. (for the purpose of this documentation we will call this memory storage STORAGE)

Then in the storage it will look for variable ENCODEMASK if not found then it will look in general env vars available to that process named ENCODEMASK (so --mask flag value is the name of the env var which holds the regex pattern)

it will take value from that env var and try to parse it to regular expression.

if error throw clear error

then it will iterate over STORAGE and using that regex it will filter out env vars to new set of env vars - keep only those to which regex points to.

Then it will try to create .env file and keep entire format like in .env.tmp

It will only change values for env vars pointed out by regex: replace original values with encrypted form.
Rest of the file (regular `#` comments, blank lines, non-matched vars) must be copied as-is to the target file.

When decrypting: swap encrypted values with decrypted values in the target file, rest copied as-is.

**`##` comment encryption** — in addition to matched env vars, encrypt lines starting with `##`:
- Encrypt any line starting with `##` (after optional leading spaces/tabs) that contains at least one non-whitespace character after `##`.
- Keep `##` and any whitespace between `##` and the first non-whitespace character in plaintext; encrypt only the payload after that whitespace. Format: `##<original_spaces><prefix><base64>`.
- Extra `#` immediately after `##` (e.g. `###`) — treat extra `#` as payload (encrypt it).
- Lines with only `##` + spaces/tabs — leave untouched in both encrypt and decrypt.
- Decrypt restores original line exactly (prefix kept, payload decrypted back).

before you start implementation we have to also think about how we will encode these env vars.

Use AES-256-GCM (import necessary libraries). Encrypted values must be base64-encoded.

Final value format in the output file:

```
ENVVAR="securenv:base64encryptedvalue"
```

Default prefix is `securenv:` but can be overridden with `--prefix <value>` argument.

No idempotency handling needed - always encrypt or always decrypt in one pass. If someone encrypts an already-encrypted file that is allowed.


We could also implement this library to handle

```sh
./bin/arm64 encrypt .env.tmp .env --mask ENCODEMASK --password-env "NAMEOFENVVARWITHPASSOWRD"
```

this way we tell our program to read password from env var NAMEOFENVVARWITHPASSOWRD

(env var have to exist and not be empty, otherwise throw with clear error message)

Obviously it will have to be able to decrypt in two modes too

```sh
./bin/arm64 decrypt .env .env.tmp --mask ENCODEMASK --password "supersecretpassword"
./bin/arm64 decrypt .env .env.tmp --mask ENCODEMASK --password-env "NAMEOFENVVARWITHPASSOWRD"
```

Target file should be always overwritten in both encrypt and decrypt mode.

If target file's parent directory does not exist - throw a clear error.

--mask is optional. if not provided then all env vars are subject to be encrypted/decrypted

--prefix is optional. defaults to `securenv:`

--stdout is optional boolean flag (default false). When provided, instead of writing to the target file, output the result to stdout. When --stdout is used, target file argument must NOT be provided - throw a clear error if it is.

Implement --help flag which prints compact usage info listing all arguments and exits. No other action performed.

Throw clear errors (specifying env var name and raw value) when encryption or decryption unsuccessful.

When throwing errors always format them like so (with prefix indicating program origin):

```
securenv error: <error_message>
```


obviously for all error cases exit 1, for success exit 0

do all in stages:
- first let's work on the rust setup (also implement compile.sh)
- then install all necessary libraries/crates (Cargo.toml dependencies) needed for AES-256-GCM, CLI arg parsing, etc. - confirm they resolve correctly before moving on
- then once I confirm we will move on to implementation in src/ and E2E tests in test/ directory executed by running test.sh script
  - generate mostly end to end test cases in separate directories like test/test001/.. test/test002/.. with everything what is needed to test all major cases -> separate .env.tmp in each test/** directory
  - each test/test[NNN] directory must contain an EXPLANATION.md describing what that particular case is meant to test
  - all tests should use the mac arm64 binary (bin/arm64)
  - test cases:
    - if it will keep the format of the file the same as source if it will only encrypt values
    - if it will respect ENCODEMASK
    - separate test for malformed ENCODEMASK
    - if source file exist or not when called `./bin/arm64 encrypt .env.fileisnotthere .env --mask ENCODEMASK --password-env "NAMEOFENVVARWITHPASSOWRD"`
    - if target file's parent directory does not exist
    - roundtrip test: encrypt → decrypt → verify decrypted values match original plaintext values
    - `##` comment encryption: ENCODEMASK non-empty, vars matching/outside mask, all `##` variants (no space, multiple spaces, tabs, leading spaces/tabs before `##`, `###`/`####`, whitespace-only ignored lines), round-trip
- then we will have to design .github/workflows/rust.yml to build in cicd
  - it will have to first build using compile.sh
  - cross-compilation for Linux (x64/arm64) and Mac (arm64/x64) is needed only in the pipeline - use whatever toolchain/docker approach is appropriate for that
  - then it will have to run test.sh to test if the program works as intended
  - publish binaries only when a git tag is created (tag-based release)
  - create DEV.md explaining to the developer how the publishing/release process works (how to create a tag, what the pipeline does, where binaries end up)
- Then create README.md to present the project idea with how to use it in compact way.
  - simple explanation what program is designed for - keep it short
  - then examples of use in README.md too
