#!/usr/bin/env sh

docker network create testnet

PORT1=26656
PORT2=26657
PORT3=8545

for i in $(seq 0 3); do
	export NODE_ID=${i}
	export PORT1
	export PORT2
	export PORT3
	export CMT_NODE_ADDR=192.167.10.$((${i}+2))
	export FMT_NODE_ADDR=192.167.10.$((${i}+6))
	export ETHAPI_NODE_ADDR=192.167.10.$((${i}+10))
	docker compose -f ./infra/docker-compose.yml -p testnet_node_${i} up -d &
	PORT1=$((PORT1+3))
	PORT2=$((PORT2+3))
	PORT3=$((PORT3+1))
done

wait $(jobs -p)