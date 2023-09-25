#!/usr/bin/env sh

if [ $# -ne 1 ]; then
  echo "usage: $0 (start|stop)"
  exit 1
fi

PORT1=26656
PORT2=26657
PORT3=8545

ACTION=

case $1 in
  start) ACTION="up -d" ;;
  stop)  ACTION="down" ;;
  *)
    echo "usage: $0 (start|stop)"
    exit 1
    ;;
esac

if [ "$1" == "start" ]; then
  docker network create --subnet 192.167.10.0/16 testnet
fi

for i in $(seq 0 3); do
	export NODE_ID=${i}
	export PORT1
	export PORT2
	export PORT3
	export CMT_NODE_ADDR=192.167.10.$((${i}+2))
	export FMT_NODE_ADDR=192.167.10.$((${i}+6))
	export ETHAPI_NODE_ADDR=192.167.10.$((${i}+10))
	docker compose -f ./infra/docker-compose.yml -p testnet_node_${i} $ACTION &
	PORT1=$((PORT1+3))
	PORT2=$((PORT2+3))
	PORT3=$((PORT3+1))
done

wait $(jobs -p)

if [ "$1" == "stop" ]; then
  docker network rm -f testnet
fi