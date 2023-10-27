# Persistent shell session

```bash
byobu
sudo -i  # for root
```

# Installing system deps

Install nginx to serve the SetupOS image
```bash
sudo apt install -y nginx
```

# Cloning the repo with the scripts for the remote deployment

```bash
cd $HOME
git clone git@gitlab.com:dfinity-lab/teams/node-team/bmc-remote-deployment.git
```

# Prepare the machines - BIOS settings and firmware

Firmware needs to be updated to the latest version and PXE boot needs to be disabled to prevent machines from booting to MaaS.

(note: I commented out the nodes that have been successfully deployed)
```bash
IPS=(
# 10.10.148.193    # dll01
# 10.10.148.197    # dll02
# 10.10.148.195    # dll03
# 10.10.148.182    # dll04
# 10.10.148.187    # dll05
# 10.10.148.198    # dll06
# 10.10.148.172    # dll07
# 10.10.148.189    # dll08
# 10.10.148.181    # dll09
# 10.10.148.178    # dll10
# 10.10.148.186    # dll11
# 10.10.148.196    # dll12
# 10.10.148.175    # dll13
# 10.10.148.184    # dll14
# 10.10.148.185    # dll15
# 10.10.148.174    # dll16
# 10.10.148.190    # dll17
# 10.10.148.173    # dll18
# 10.10.148.199    # dll19
# 10.10.148.180    # dll20
10.10.148.194    # dll21
# 10.10.148.183    # dll22
# 10.10.148.179    # dll23
# 10.10.148.192    # dll24
# 10.10.148.176    # dll25
# 10.10.148.177    # dll26
# 10.10.148.191    # dll27
# 10.10.148.188    # dll28
)
USER=root
PASSWORD=PJGHbfdDZAdtZwCWgYJ8

cd "$HOME/bmc-remote-deployment/iDRAC-Redfish-Scripting/Redfish Python"

# Reset iDRACs if needed. Example where this helps is if a job gets stuck and needs kicking.
# Otherwise, this step is not necessary although it's not harmful. It just takes a while to reboot iDRAC.
date
for IP in "${IPS[@]}"; do
    python3 ./ResetIdracREDFISH.py -ip $IP -u $USER -p $PASSWORD &
done
wait

# Delete all iDRAC jobs. Example where this helps is if a job gets stuck and needs kicking.
# Also, consider running this only on specific machines by commenting out the list of IPs above.
date
for IP in "${IPS[@]}"; do
    python3 ./DeleteJobQueueREDFISH.py -ip $IP -u $USER -p $PASSWORD --clear &
done
wait

# Set the DNS configuration on iDRAC. This needs to be run once, since Dell machines often do not have this set up.
date
for IP in "${IPS[@]}"; do
    python3 ./SetIdracLcSystemAttributesREDFISH.py -ip $IP -u $USER -p $PASSWORD --set idrac --attribute-names NICStatic.1.DNSDomainFromDHCP,IPv4Static.1.DNS1,IPv4Static.1.DNS2  --attribute-values "0,1.1.1.1,1.0.0.1" &
done
wait

for IP in "${IPS[@]}"; do
    python3 ./InsertEjectVirtualMediaREDFISH.py -ip $IP -u $USER -p $PASSWORD --action eject --index 1 &
done
wait

date
for IP in "${IPS[@]}"; do
    python3 ./GetSetPowerStateREDFISH.py -ip $IP -u $USER -p $PASSWORD --set On &
done
wait

# Update the firmware. This needs to be completed on each machine. Notes:
# - the update job may take very long time to complete (e.g. 15-30 minutes)
# - the update job may get stuck and you may need to delete all iDRAC jobs in that case (look above)
# - the only way to know (that I found) that the firmware is already at the latest version is to check the output messages, where you should see an error message that the firmware upgrade failed _possibly_ because it's running the latest version.
date
for IP in "${IPS[@]}"; do
    python3 ./InstallFromRepositoryREDFISH.py -ip $IP -u $USER -p $PASSWORD --install --shareip downloads.dell.com --sharetype HTTPS --applyupdate True --rebootneeded True &
done
wait

# Disable PXE boot in BIOS. This needs to run once successfully on all machines, otherwise the machines will try to boot to MaaS.
# Note that it's fairly hard to figure out if the settings have been applied correctly, since a machine reboot is needed to apply them.
# You may need to reboot the machines once and then run the command again if you see some machines still booting to MaaS.
# The easiest way I found to reliably check the current settings is to login in iDRAC and check there.
date
for IP in "${IPS[@]}"; do
    python3 ./GetSetBiosAttributesREDFISH.py -ip $IP -u $USER -p $PASSWORD \
       --attribute-names "BootMode,PxeDev1EnDis,CpuMinSevAsid,LogicalProc,NumaNodesPerSocket,PcieEnhancedPreferredIo,PciePreferredIoBus,ProcVirtualization,SriovGlobalEnable,ErrPrompt" \
       --attribute-values "Uefi,Disabled,253,Enabled,0,Enabled,Enabled,Enabled,Enabled,Disabled" --duration-time 1200 --reboot &
done
wait

# Get BIOS configuration for the nodes. This is a lot of data, and you likely do not have to run it.
date
for IP in "${IPS[@]}"; do
    python3 ./GetSetBiosAttributesREDFISH.py -ip $IP -u $USER -p $PASSWORD \
        --get-attribute PxeDev1EnDis &
done
wait

# Power on all machines.
date
for IP in "${IPS[@]}"; do
    python3 ./GetSetPowerStateREDFISH.py -ip $IP -u $USER -p $PASSWORD --set On &
done
wait
```

# Prepare the SetupOS image

Instructions at https://gitlab.com/dfinity-lab/teams/node-team/bmc-remote-deployment
```bash
cd $HOME
GIT_REV=3bcccef07408921fe849c92dd2437adc157ef9c3
curl -LO https://download.dfinity.systems/ic/$GIT_REV/setup-os/disk-img/disk-img.tar.gz
tar -xvf disk-img.tar.gz

cd $HOME
LOOPDEV=/dev/loop3001
sudo losetup --partscan --show ${LOOPDEV} disk.img
mkdir -p /tmp/mounts/setupos-config
sudo mount ${LOOPDEV}p3 /tmp/mounts/setupos-config

# update config at /tmp/mounts/setupos-config
mkdir -p $HOME/setupos-config-mr1
cat >| $HOME/setupos-config-mr1/config.ini <<_EOF
# Please update the template/example below.
#
# If you need help, please do not hesitate to contact the
# Internet Computer Association.
#
ipv6_prefix=2a0b:21c0:b002:2
ipv6_subnet=/64
ipv6_gateway=2a0b:21c0:b002:2::1
_EOF

# !!!!!!!!!!!!!!!
# !!!IMPORTANT!!!
# !!!!!!!!!!!!!!!
# download the latest version of $HOME/setupos-config-mr1/ssh_authorized_keys/admin from
# https://gitlab.com/dfinity-lab/private/infrasec/prod-ssh/-/blob/main/mercury/admin
# as instructed in the BMC repo https://gitlab.com/dfinity-lab/teams/node-team/bmc-remote-deployment/-/blob/master/README.md
mkdir -p $HOME/setupos-config-mr1/ssh_authorized_keys/
vi $HOME/setupos-config-mr1/ssh_authorized_keys/admin

# !!!!!!!!!!!!!!!
# !!!IMPORTANT!!!
# !!!!!!!!!!!!!!!
# download the latest version of $HOME/setupos-config-mr1/node_operator_private_key.pem from 1password
sudo cp $HOME/setupos-config-mr1/config.ini /tmp/mounts/setupos-config/config.ini
sudo cp $HOME/setupos-config-mr1/ssh_authorized_keys/admin /tmp/mounts/setupos-config/ssh_authorized_keys/admin
sudo cp $HOME/setupos-config-mr1/node_operator_private_key.pem /tmp/mounts/setupos-config/node_operator_private_key.pem
# Unmount and proceed
sudo umount /tmp/mounts/setupos-config
sudo losetup -d ${LOOPDEV}
sudo cp $HOME/disk.img /var/www/html/mr1-setupos.img
sudo chmod 0644 /var/www/html/mr1-setupos.img
```

Create a list of nodes to be deployed:
```bash
cat >| $HOME/bmc-remote-deployment/deploy-mr1.csv <<_EOF
10.10.148.193,$USER,$PASSWORD
10.10.148.197,$USER,$PASSWORD
10.10.148.195,$USER,$PASSWORD
10.10.148.182,$USER,$PASSWORD
10.10.148.187,$USER,$PASSWORD
10.10.148.198,$USER,$PASSWORD
10.10.148.172,$USER,$PASSWORD
10.10.148.189,$USER,$PASSWORD
10.10.148.181,$USER,$PASSWORD
10.10.148.178,$USER,$PASSWORD
10.10.148.186,$USER,$PASSWORD
10.10.148.196,$USER,$PASSWORD
10.10.148.175,$USER,$PASSWORD
10.10.148.184,$USER,$PASSWORD
10.10.148.185,$USER,$PASSWORD
10.10.148.174,$USER,$PASSWORD
10.10.148.190,$USER,$PASSWORD
10.10.148.173,$USER,$PASSWORD
10.10.148.199,$USER,$PASSWORD
10.10.148.180,$USER,$PASSWORD
10.10.148.194,$USER,$PASSWORD
10.10.148.183,$USER,$PASSWORD
10.10.148.179,$USER,$PASSWORD
10.10.148.192,$USER,$PASSWORD
10.10.148.176,$USER,$PASSWORD
10.10.148.177,$USER,$PASSWORD
10.10.148.191,$USER,$PASSWORD
10.10.148.188,$USER,$PASSWORD
_EOF
```

Serve the image with http (nginx) and deploy node(s):
```bash
cd $HOME/bmc-remote-deployment
IP_ADDR=$(ip a s | grep "inet " | grep "scope global" | head -n1 | awk '{print $2}' | cut -d/ -f1)
curl --head http://$IP_ADDR/mr1-setupos.img
head -n1 $HOME/bmc-remote-deployment/deploy-mr1.csv > deploy-mr1-node-1.csv
poetry run ./deploy.py --csv-filename deploy-mr1-node-1.csv --network-image-url http://$IP_ADDR/mr1-setupos.img --wait-time 20
```

Once the first node is deployed, you can use the CSV file for the other nodes. For instance, `deploy-mr1-node-2.csv`, or for multiple nodes, say `deploy-mr1-node-7+.csv`.

After all nodes are deployed, make sure you turn off nginx, since it serves an image with the node operator private key embedded.

```
sudo systemctl stop nginx
sudo systemctl disable nginx
```

Now you need to inject the admin ssh keys into the GuestOS.
You can get the GuestOS IPs from the internal dashboard or from FactsDB.
```bash
HOST_IPv6=(
2a0b:21c0:b002:2:6800:e2ff:fe91:32d6	
2a0b:21c0:b002:2:6800:55ff:fe4c:31c	
2a0b:21c0:b002:2:6800:25ff:feee:6528	
2a0b:21c0:b002:2:6800:30ff:fe19:53e0	
2a0b:21c0:b002:2:6800:25ff:fe94:c7fe	
2a0b:21c0:b002:2:6800:62ff:fe9e:9309	
2a0b:21c0:b002:2:6800:eff:fe20:49d5	
2a0b:21c0:b002:2:6800:73ff:fe29:d342	
2a0b:21c0:b002:2:6800:40ff:feb9:c5ee	
2a0b:21c0:b002:2:6800:d3ff:fe3f:5032	
2a0b:21c0:b002:2:6800:99ff:fe1e:8f79	
2a0b:21c0:b002:2:6800:1fff:feea:6f5f	
2a0b:21c0:b002:2:6800:2dff:feee:7fc5
2a0b:21c0:b002:2:6800:76ff:feea:2388	
2a0b:21c0:b002:2:6800:b4ff:fe27:8a9e	
2a0b:21c0:b002:2:6800:9dff:fe3e:8f90	
2a0b:21c0:b002:2:6800:cdff:fe98:ffa0	
2a0b:21c0:b002:2:6800:b6ff:fe37:2a67	
2a0b:21c0:b002:2:6800:e0ff:fe34:b161	
2a0b:21c0:b002:2:6800:30ff:fee0:d46	
2a0b:21c0:b002:2:6800:abff:fe47:f635	
2a0b:21c0:b002:2:6800:aeff:fef2:7ccf	
2a0b:21c0:b002:2:6800:e5ff:fecc:efe4	
2a0b:21c0:b002:2:6800:b5ff:fe6b:a4d1	
2a0b:21c0:b002:2:6800:78ff:fe9d:1fe0	
2a0b:21c0:b002:2:6800:dcff:fe66:e1c3	
2a0b:21c0:b002:2:6800:10ff:febd:8632	
)
# dll21 is excluded from the list above since it failed to come up with a good IPv6 address

cat >| inject-admin.sh <<_EOF
#!/bin/bash

systemctl stop guestos.service
losetup -P /dev/loop0 /dev/hostlvm/guestos

mount /dev/loop0p3 /mnt
cp /var/lib/admin/.ssh/authorized_keys /mnt/accounts_ssh_authorized_keys/admin
chmod 0644 /mnt/accounts_ssh_authorized_keys/admin
umount /mnt

losetup -d /dev/loop0
systemctl start guestos.service
_EOF

for IP in "${HOST_IPv6[@]}"; do
    set -x
    # The -F /dev/null disables the use of any files in ~/.ssh for use as an identity. This will only leave the option of the
    # yubikey to connect
    scp -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -F /dev/null -6 inject-admin.sh admin@"[$IP]":
    ssh -tt -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -F /dev/null -6 admin@$IP "chmod +x inject-admin.sh && sudo ./inject-admin.sh"
    set +x
done
```
