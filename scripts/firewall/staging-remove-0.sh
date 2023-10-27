#!/bin/bash

./ic-admin --secret-key-pem=$HOME/.config/dfx/identity/bootstrap-super-leader/identity.pem --nns-url="http://[2600:3004:1200:1200:5000:62ff:fedc:fe3c]:8080" propose-to-remove-firewall-rules --proposer 49 replica_nodes 1 $(./ic-admin --secret-key-pem=$HOME/.config/dfx/identity/bootstrap-super-leader/identity.pem --nns-url="http://[2600:3004:1200:1200:5000:62ff:fedc:fe3c]:8080" propose-to-remove-firewall-rules --test --proposer 49 replica_nodes 1 "none" | tail -n 1 | cut -d' ' -f2 | tr -d '"')
