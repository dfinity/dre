If we ignore the question of lockouts, our task is like this:

We start with an allocation of bare metal hosts:

```
[parent_nns]
dc1-m001
dc2-m007


[child_nns]
dc1-m005
dc7-m047

[app_subnet_1]
dc2-m007
dc2-m007

[app_subnet_2]
dc2-m007
dc2-m007

[app_subnet_3]
dc2-m007
dc2-m007
```


We neeed to map this to a child inventory, first adding IP addresses, then principal IDs:

```
[parent_nns]
dc1-m001 ipv6=xx:xx:xx::xx
dc2-m007 ipv6=xx:xx:xx::xx
[child_nns]
...
```

Then:

```
[parent_nns]
dc1-m001 ipv6=xx:xx:xx::xx principal="5r3ha-sft6i-7zidr-s3abw-s3gmt-hna5a-f2ubq-djjbp-c5e54-obejo-qae"
dc2-m007 ipv6=xx:xx:xx::xx principal="gktih-2rd7p-yqnre-krirh-jfjln-3ivho-zcsf6-u3zrg-jk5mc-tj7yn-6ae"
[child_nns]
...
```


This currently happens in several steps.

* Define bare metal allocations
* Collect the serial numbers of all the bare metal hosts, so create a guest inventory with IPv6 addresses.
* (Not relevant for the inventory: Deploy root node; this is is a classical nix machine.)
* Deploy the parent and child NNS subnets.  Gradually, as these register with the parent NNS, fill in the principal IDs of the parent and child NNS nodes.
* (Not relevant for the inventory: Configure the NNS nodes through the root NNS node.)
* Deploy the app subnets.  Gradually, as these register with the child NNS, fill in the principal IDs of the parent and child NNS nodes.
* (Not relevant for the inventory: Configure the app nodes through the child NNS.)


There is an additional dimension that some nodes in each section are lockout nodes.  Lockout nodes have different media images, have to be prepopulated with the lockout script, and are not started by ansible.  In theory each node could have a flag indication whether to lock out or not but so far it has actually been easier to have a separate section for lockout nodes.

Thus the inventory actually looks like this:

```
[parent_nns]
dc1-m001

[parent_nns_lockout]
dc2-m007


[child_nns]
dc1-m005

[child_nns_lockout]
dc7-m047


[app_subnet_1]
dc2-m007

[app_subnet_1_lockout]
dc2-m007

etc
```

and the deploynent steps are:

* Define bare metal allocations
* Maas wipe all nodes (apart from the root node, not listed above)
* Collect the serial numbers of all the bare metal hosts, so create a guest inventory with IPv6 addresses.

* Lockout NNS nodes:
  * Deploy the parent and child NNS lockout nodes.  (This is ansible for the initial configuration of the nodes, again for the lockout script and again to install the guest, however these three steps could be combined.  All three ansible scripts would have to land in master of tthis, or at least in the rc3 branch but master is preferable.)
  * Do an HSM dance:
    * Insert HSMs with free node allocations into the nodes.
    * Gradually, as these register with the parent NNS, fill in the principal IDs of the parent and child NNS nodes.
* DFINITY NNS nodes:
  * Deploy the parent and child NNS lockout nodes.
  * Do a simpler HSM dance:
    * Node allocation rules still apply, however HSMs can be pre-inserted or re-inserted if needed.
    * Gradually, as these register with the parent NNS, fill in the principal IDs of the parent and child NNS nodes.
* Configure nodes throuth the root NNS.

* App nodes:  Roughly similar to the NNS nodes.
