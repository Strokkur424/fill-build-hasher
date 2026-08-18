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
project,version,build,file_name,download_key,md5,sha1,sha512
velocity,4.1.0-SNAPSHOT,16,velocity-4.1.0-SNAPSHOT-16.jar,server:default,59dc758ad014586c1a7118914d1baf19,d423795ebc2de55fa5c07bc1d92f27b45752a3fd,72a8b4b9fc65272a88d9e2da035e012a90846218ff0346d26fc8519f17d6237eca34fbe8c5a486444d4169813bd609a79f62701a28d291459e329cd67bd871b2
velocity,4.1.0-SNAPSHOT,15,velocity-4.1.0-SNAPSHOT-15.jar,server:default,39e90ee923d52a32edef6e2cef484019,5cc128d468cf5f3507c83c3b4b13beec91825b97,55da593ee6a95896857a5d04434caa23040af94da62907de85956bee025ed636412a1c00a6b7bf2b6a80da298e9927cc284e0b81cec9f2d1d9b84d6391947301
velocity,4.1.0-SNAPSHOT,20,velocity-4.1.0-SNAPSHOT-20.jar,server:default,936171cc540f98545a0f3bf348528bd6,f9110574c0dc57b6292647dd6b39ebccfd1d66a9,1672e88e985b282ddf76a84dbacd1872b6b9212e78351736f6d3e4d9452a4d9d8d14adeff8ef6f6d7c7ff587ab6c6d752ed8a518c7a5c89c67aaff3663104bc5
velocity,4.1.0-SNAPSHOT,21,velocity-4.1.0-SNAPSHOT-21.jar,server:default,87261027cb8bb4cd66a0c3bd82fccd99,21270994a99446581ce03de60a40570642c152f8,a21aca842342a48190ff052e62fc624a04e95863e5e214f590048f55e51a4865be955c5b18efe3121f8f9a5b86ea790b4bd1d97a409d73ab4c1a1d4ccc8299ed
velocity,4.1.0-SNAPSHOT,19,velocity-4.1.0-SNAPSHOT-19.jar,server:default,563739abdbdf6dfad69ae57ddb2f2663,0061f4f70050843d08c45587a14e7474e2ad8618,07866624447f8a00e4d4a067f2fac004af4e469e2d16545a48dad3fd008e64dd89e8b309569af2fbf4ec309e3da27df4d5bdb7bf6c682746a08803a73fcca2cd
```
