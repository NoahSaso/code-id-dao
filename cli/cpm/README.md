# CPM: Cosmwasm Package Manager

![cpm logo](https://cdn.discordapp.com/attachments/975507980780994600/992154202367328327/cosmos-pak-man.png)

CLI tool for [cw-contract-registry smart contract](../../contracts/cw-code-id-registry/README.md).

CPM cli can install packages from any `cw-contract-registry` instance. Each organization is responsible for instantiating their own instance of `cw-contract-registry` and registering their own package versions.

Use the [cpm site](TODO) to search for packages across all orgs.

## Installation

`cargo install cosmwasm-package-manager`

## Usage 

| Command  | Description |
| ------------- | ------------- |
| `cargo run cpm --version` | Print cpm version  |
| `cargo run cpm help` | See help information  |
| `cargo run cpm init` | Create default `cpm.yaml` config file. |
| `cargo run cpm install` | Download the code ids for each contract dependency stored in `cpm.yaml` |
| `cargo run cpm upgrade` | Upgrade to the latest versions for each contract dependency stored in `cpm.yaml` |
| `cargo run cpm release <VERSION>` | Bump all of the packages versions to `VERSION` in `cpm.yaml`. Optimize / store the packages in `cpm.yaml` to `--chain_id`. Then make a DAO proposal to register the contract versions |


### Release

`cpm release` will create a proposal in the configured `Release.dao_addr`'s DAO to register the new smart contract versions.

* Bump smart contract version to `<VERSION>` for each release package config in `cpm.yaml`
* Build and optimize the wasms in `<CARGO_PATH>`
* Store those new optimized wasms on `<CHAIN_ID>`
* Make a DAO proposal to register the new contract versions


### OS Keyring Password

Currently cpm only lets you use an existing OS Keyring password, but it will eventually support creating and deleting Keyring passwords through the cli.

For MacOS users follow [these steps](https://support.apple.com/guide/keychain-access/add-a-password-to-a-keychain-kyca1120/mac) to add your private key mnemonic to your OS Keychain. Be sure to use `cpm` for the `Where` section, and your `<KEY_NAME>` will be whatever you put in `Account`.
