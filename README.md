# 🚢 fill-build-hasher

A simple CLI tool written in Rust that can iterate through all builds from a [fill](https://github.com/PaperMC/fill)-provided project and compute their MD5
hash.

## Usage

```bash
./fill-build-hasher --help
Usage: fill-build-hasher [OPTIONS] --projects <PROJECTS>

Options:
      --limit <LIMIT>              The max amount of builds to download. Can be used for testing purposes
      --buffer-size <BUFFER_SIZE>  The number of concurrent downloads to attempt [default: 32]
  -r, --retries <RETRIES>          The amount of retries for each fill API call, if it fails [default: 5]
      --file-path <FILE_PATH>      The target *.csv file to write to
  -p, --projects <PROJECTS>        The project to iterate all builds in
      --endpoint <ENDPOINT>        The fill endpoint to use [default: https://fill.papermc.io/v3]
      --skip-timeout               Whether to skip the timeout between different project hashing
  -h, --help                       Print help
```

## Example usage

To compute the MD5 for every build inside the projects `folia`, `paper`, `waterfall`, and `velocity` into a single CSV file `md5.csv`, you can use this command:

```bash
./fill-build-hasher --projects folia paper waterfall velocity --file-path md5.csv
```

## Target file format

The output is formatted as a `csv` file. It looks like so:

```csv
project,version,build,file_name,md5
velocity,4.1.0-SNAPSHOT,21,velocity-4.1.0-SNAPSHOT-21.jar,87261027cb8bb4cd66a0c3bd82fccd99
velocity,4.1.0-SNAPSHOT,20,velocity-4.1.0-SNAPSHOT-20.jar,936171cc540f98545a0f3bf348528bd6
velocity,4.1.0-SNAPSHOT,16,velocity-4.1.0-SNAPSHOT-16.jar,59dc758ad014586c1a7118914d1baf19
velocity,4.1.0-SNAPSHOT,15,velocity-4.1.0-SNAPSHOT-15.jar,39e90ee923d52a32edef6e2cef484019
velocity,4.1.0-SNAPSHOT,19,velocity-4.1.0-SNAPSHOT-19.jar,563739abdbdf6dfad69ae57ddb2f2663
```
