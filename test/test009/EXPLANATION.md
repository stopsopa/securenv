Tests the `<dollar>` placeholder replacement and `/.../` delimiter handling in ENCODEMASK.
Variables ending with matched suffix (e.g. `JWT_EXPIRE_SECONDS`, `PROJECT_NAME`, `PUBLIC_KEY`) should be encrypted, while others (like `PROJECT_NAME_EXTRA`) should remain untouched.
