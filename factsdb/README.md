# Install

```
pip3 install -r requirements.txt
```

# Usage

To list all guests in the `mercury` deployment (default deployment):
```
./main.py --guests
```

To list all principals
```
./main.py --principals
```

To list all physical machines and their serial numbers
```
./main.py --physical
```

To list all subnets and list of subnet members
```
./main.py --subnets
```

To list the nodes from the unused subnet:
```
./main.py --subnets | grep unassigned | grep -oP '(?<=members=).*\b' | tr ',' '\n' | sort
```

To list subnets and replica revisions in them
```
./main.py --subnet-replica-revisions
```

All previous commands can be used together with `--refresh` to ensure that the latest data is used. For instance:
```
./main.py --refresh --guests
```

It's also possible to only refresh the data:
```
./main.py --refresh
```

# Usage for other networks

For all commands below, first set the target deployment, for example:
```
DEPLOYMENT=bootstrap
```

Refresh missing facts for physical hosts
```
./main.py --deployment $DEPLOYMENT --physical --refresh
```

Refresh missing facts for node principals
```
./main.py --deployment $DEPLOYMENT --principals --refresh
```

Refresh missing facts for guests (nodes) hosts
```
./main.py --deployment $DEPLOYMENT --guests --refresh
```

# Useful commands

One-off command on all deployment physical hosts:
```
INCLUDE_ALL_PHYSICAL_HOSTS=1 ansible -i testnet/env/$DEPLOYMENT/hosts physical_hosts -m shell -a "echo"
```

One-off command on all deployment physical hosts, as root:
```
INCLUDE_ALL_PHYSICAL_HOSTS=1 ansible -i testnet/env/$DEPLOYMENT/hosts physical_hosts -m shell --become -a "echo"
```
