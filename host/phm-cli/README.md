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
    console       I2C Write console mode
    read          Read count bytes from the given address
    write         Write bytes to the given address
    write-read    Write-Read bytes to and from the given address
```

### I2C Write console mode (`phm-cli i2c console`)

```
phm-cli-i2c-console
Write bytes over I2C.
Provide a comma separated list of bytes (hex) then press enter to execute.

USAGE:
    phm-cli i2c console -a <ADDRESS>

OPTIONS:
    -a <ADDRESS>                  The address to write to
    -h, --help                    Print help information
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
    --read-ct <READ_COUNT>        Number of bytes to read
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

## SPI Commands (`phm-cli spi`)

```
phm-cli-spi
Commands for SPI communication

USAGE:
    phm-cli spi <SUBCOMMAND>

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    help          Print this message or the help of the given subcommand(s)
    console       SPI Transfer console mode
    transfer      Write and read bytes over SPI
    write         Write bytes over SPI
```

### SPI Transfer console mode (`phm-cli spi console`)

```
phm-cli-spi-console
Write and read bytes over SPI.
Provide a comma separated list of bytes (hex) then press enter to execute.

USAGE:
    phm-cli spi console

OPTIONS:
    -h, --help    Print help information
```

### SPI Transfer (`phm-cli spi transfer`)

```
phm-cli-spi-transfer
Write and read bytes over SPI

USAGE:
    phm-cli spi transfer --write <WRITE_BYTES>

OPTIONS:
    -b, --write <WRITE_BYTES>    Bytes to write to SPI. Should be given as a comma-separated
                                 list of hex values. For example: "0xA0,0xAB,0x11"
    -h, --help                   Print help information
```

### SPI Write (`phm-cli spi write`)

```
phm-cli-spi-write
Write bytes over SPI

USAGE:
    phm-cli spi write --write <WRITE_BYTES>

OPTIONS:
    -b, --write <WRITE_BYTES>    Bytes to write to SPI. Should be given as a comma-separated
                                 list of hex values. For example: "0xA0,0xAB,0x11"
    -h, --help                   Print help information
```
