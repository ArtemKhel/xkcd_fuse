### xkcd FUSE
Mount xkcd comic as a filesystem using FUSE.

```text
xkcd
├── xkcd_3073
│   ├── xkcd_3073.alt
│   ├── xkcd_3073.num
│   ├── xkcd_3073.png
│   ├── xkcd_3073.release_date
│   ├── xkcd_3073.title
│   └── xkcd_3073.transcript
└── ...
```

##### Usage
```text
Usage: xkcd_fuse [OPTIONS]

Options:
      --db <DB_PATH>         Path to the SQLite database file [default: ./db.sqlite]
      --mount <MOUNT_POINT>  Mount point for the XkcdFS [default: ./xkcd/]
      --start <START>        Start of the range to fetch [default: 4294967295]
      --end <END>            End of the range to fetch [default: 4294967295]
  -h, --help                 Print help
  -V, --version              Print version
```

##### Build and run

For users of superior package manager (Nix):

```sh 
nix run -- <args>
```
to simply run and
```sh
nix develop
```
to drop into devshell with all the dependencies

