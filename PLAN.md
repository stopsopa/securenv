I need rust setup which will do compilation of code from directory src/ and produce binaries in bin/ for x64 and for arm64. (bin/arm64 and bin/x64).

(tell me also how to install rust tools on my mac btw)

then having that create source code in src/ for program as follow

I need program which once executed like this

```sh
./bin/x64 encrypt .env.tmp .env --mask ENCODEMASK --password "supersecretpassword"
```

that program will open file .env.tmp then it will read all it's env vars to te form of key value native rust data structure and keep it all of these in memory. (for the purpose of this documentation we will call this memory storage STORAGE)

Then in the storage it will look for variable ENCODEMASK if not found then it will look in general env vars available to that process ENCODEMASK

it will take value from that env var and try to parse it to regular expression.

if error throw clear error

then it will iterate over STORAGE and using that regex it will filter out env vars to new set of env vars - keep only those to which regex points to.

Then it will try to create .env file and keep entire format like in .env.tmp

it will only change values for env vars pointed out by regex to encrypted form

before you start implementation we have to also think about how we will encode these env vars.

Ideally I would use some secure version of aes256 (import necessary libraries for this implementation).


We could also implement this library to handle 

```sh
./bin/x64 encrypt .env.tmp .env --mask ENCODEMASK --password-env "NAMEOFENVVARWITHPASSOWRD"
```

this way we tell our program to read password from env var NAMEOFENVVARWITHPASSOWRD

(env var have to exist and not be empty, otherwise throw with clear error message)

Obviously it will have to be able to decrypt in two modes too

```sh

./bin/x64 decrypt .env .env.tmp --mask ENCODEMASK --password "supersecretpassword"
./bin/x64 decrypt .env .env.tmp --mask ENCODEMASK --password-env "NAMEOFENVVARWITHPASSOWRD"

```

--mask is optional. if not provided then all env vars are subject to be encrypted/decrypted

Throw clear errors (specifying env var name and raw value) when encryption or decryption unsuccessful.

When throwing errors always format them like so (with prefix indicating program origin):

```
securenv error: <error_message>
```

obviously for all error cases exit 1, for success exit 0

do all in stages:
- first let's work on the rust setup
- then once I confirm we will move on to implementation in src/
- then we will have to design .github/workflows/rust.yml to build in cicd to also publish binaries for each version on github to be available for download
- prepare entire setup to build for linux (x64/arm64), mac (arm64/x64)



