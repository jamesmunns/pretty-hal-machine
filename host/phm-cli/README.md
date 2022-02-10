# pretty hal machine cli

## Top Level (`phm-cli`)

```
phm-cli 

USAGE:
    phm-cli <SUBCOMMAND>

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    help    Print this message or the help of the given subcommand(s)
    i2c     Commands for I2C communication
```

## I2C Commands (`phm-cli i2c`)

```
phm-cli-i2c 
Commands for I2C communication

USAGE:
    phm-cli i2c <SUBCOMMAND>

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    help          Print this message or the help of the given subcommand(s)
    read          Read count bytes from the given address
    write         Write bytes to the given address
    write-read    Write-Read bytes to and from the given address
```

### I2C Read (`phm-cli i2c read`)

```
phm-cli-i2c-read 
Read count bytes from the given address

USAGE:
    phm-cli i2c read -a <ADDRESS> --read-ct <READ_COUNT>

OPTIONS:
    -a <ADDRESS>                  The address to write to
    -h, --help                    Print help information
        --read-ct <READ_COUNT>    Number of bytes to read
```

### I2C Write (`phm-cli i2c write`)

```
phm-cli-i2c-write 
Write bytes to the given address

USAGE:
    phm-cli i2c write -a <ADDRESS> --write <WRITE_BYTES>

OPTIONS:
    -a <ADDRESS>                 The address to write to
    -b, --write <WRITE_BYTES>    Bytes to write to the address. Should be given as a comma-separated
                                 list of hex values. For example: "0xA0,0xAB,0x11"
    -h, --help                   Print help information
```

### I2C Write then Read (`phm-cli i2c write-read`)

```
phm-cli-i2c-write-read 
Write-Read bytes to and from the given address

USAGE:
    phm-cli i2c write-read -a <ADDRESS> --bytes <WRITE_BYTES> --read-ct <READ_COUNT>

OPTIONS:
    -a <ADDRESS>                  The address to write to. Should be given as a hex value. For
                                  example: "0xA4"
    -b, --bytes <WRITE_BYTES>     
    -h, --help                    Print help information
        --read-ct <READ_COUNT>    Bytes to write to the address. Should be given as a comma-
                                  separated list of hex values. For example: "0xA0,0xAB,0x11"
```
