# Running Fendermint

The commands are all executed by the `fendermint` binary, which is produced from the `fendermint_app` crate,
so we have two ways to run the program:
* `./target/debug/fendermint <args>` (or wherever it's been installed)
* `cargo run -p fendermint_app -- <args>`

## Genesis

The first step we need to do is create a Genesis file which we'll pass to Tendermint,
which will pass it to Fendermint through ABCI. This Genesis file will be in JSON format,
as that is the convention with Tendermint, but we could also encode it in CBOR if we wanted.

Ostensibly the Genesis content could be coming from the parent chain itself, so the child
subnet participants don't have to go through the manual steps below, but we still have these
options to start a standalone network.

In the following sections we will create a Genesis file for a network named `test`. Start by creating a directory to hold the files:

```shell
mkdir test-network
```

### Create a new Genesis file

First, create a new `genesis.json` file devoid of accounts and validators. The `--base-fee` here is completely arbitrary.

```shell
cargo run -p fendermint_app -- genesis --genesis-file test-network/genesis.json new --network-name test --base-fee 1000
```

We can check what the contents look like:

```console
$ cat test-network/genesis.json
{
  "network_name": "test",
  "network_version": 18,
  "base_fee": "1000",
  "validators": [],
  "accounts": []
}
```

### Create some keys

Next, let's create some cryptographic key pairs we want want to use either for accounts or validators at Genesis.

```shell
mkdir test-network/keys
for NAME in alice bob charlie dave; do cargo run -p fendermint_app -- keygen --out-dir test-network/keys --name $NAME; done
```

Check that the keys have been created:

```console
$ ls test-network/keys
alice.pk  alice.sk  bob.pk  bob.sk  charlie.pk  charlie.sk  dave.pk  dave.sk

$ cat test-network/keys/alice.pk
Ak5Juk793ZAg/7Ojj4bzOmIFGpwLhET1vg2ROihUJFkq
```
