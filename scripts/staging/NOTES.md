# Commands

## Enable the root (root) password

```
virsh destroy guest


cd /var/lib/libvirt/images/

mkdir -p mnt
kpartx -a guest.img
loopdev=$(losetup -l | grep $(realpath "guest.img") | tail -n1 | awk '{print $1}')
loopdev="/dev/mapper/$(basename "$loopdev")p5"
mount $loopdev mnt/
chroot mnt bash -c 'echo root:root | chpasswd'
umount mnt
kpartx -d guest.img
rmdir mnt

virsh start

virsh console guest
```

## Get the guest mac address from the host

```
virsh dumpxml guest | xmllint --xpath 'string(/domain/devices/interface[target[@dev="vlan66"]]/mac/@address)' -
```

## Vote

Get proposal info, from anonymous user (will not return any ballot in the response):
```
icx  http://[2a00:fb01:400:100:5000:c9ff:fe2b:522e]:8080/ query  rrkah-fqaaa-aaaaa-aaaaq-cai --candid nns/governance/canister/governance.did get_proposal_info '(3)'
```

Get proposal info, from real user:
```
icx --pem ~/.config/dfx/identity/bootstrap-support/identity.pem http://[2a00:fb01:400:100:5000:c9ff:fe2b:522e]:8080/ query  rrkah-fqaaa-aaaaa-aaaaq-cai --candid nns/governance/canister/governance.did get_proposal_info '(3)'
```

Vote, the easy way:
```
icx --pem ~/.config/dfx/identity/super-leader/identity.pem http://[2a00:fb01:400:100:5000:c9ff:fe2b:522e]:8080/ update  rrkah-fqaaa-aaaaa-aaaaq-cai  forward_vote  '(49 : nat64, 1 : nat64, variant{Yes})'
```

see that the --candid option is absent here. You can add it, it does not hurt, but also does not help for the input parsing because forward_vote is on purpose excluded from the candid declaration as discouragement (we can revert that)
because there is no candid file, every type MUST be specified explicitly, hence the : nat64.

Vote, the hard way:
```
icx --pem ~/.config/dfx/identity/super-leader/identity.pem http://[2a00:fb01:400:100:5000:c9ff:fe2b:522e]:8080/ update  rrkah-fqaaa-aaaaa-aaaaq-cai manage_neuron '(record {id=opt record {id=49}; command=opt variant {RegisterVote=record {vote=1; proposal=opt record {id=1}}}})' --candid nns/governance/canister/governance.did
```

Warning: any error in the RegisterVote subcommand will not trigger an error. Instead it will silently drop it and send an empty manageneuron to the governance canister
