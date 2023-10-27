
# Update firewall rules

Updating the position 0 rules
```
pipenv run python3 propose.py --deployment mainnet --type update --position 0 --motivation "Adding a new IPv6 prefix for the SF1 InfraDC, as per https://forum.dfinity.org/t/proposal-101816-update-firewall-rules/17944"
```

Or to add boundary nodes in a new DC:
```
pipenv run python3 propose.py --deployment mainnet --type add --position 4 --motivation "Allowing the Boundary Nodes in the SE1 DC to access Replica nodes"
```
