# CPM: Cosmwasm Package Manager

Frontend CLI for [Code ID Registry Smart Contract](../../contracts/cw-code-id-registry/README.md).

## Installation

`cargo install cosmwasm-package-manager`

## Usage 

| Command  | Description |
| ------------- | ------------- |
| `cargo run cpm --version` | Print cpm version  |
| `cargo run cpm help` | See help information  |
| `cargo run cpm init` | Create default `cpm.yaml` config file. |
| `cargo run cpm install` | Download the code ids for each contract version stored in `cpm.yaml` |
| `cargo run cpm upgrade` | Upgrade to the latest versions for each contract stored in `cpm.yaml` |

## Official Code ID Registry Addresses

Use this address as the `registry_addr` in your `cpm.yaml` to use the Code ID Registry maintained by the DAO.

| Chain | Address | Link | 
| ------------- | ------------- | ------------- |
| juno mainnet | TODO | TODO
