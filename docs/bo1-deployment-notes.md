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

dll01 - already up
dll02 - broken, skip
dll03 - already up
dll04 - already up

```bash
IPS=(
# 10.10.146.192
# 10.10.146.191
# 10.10.146.188
# 10.10.146.189
10.10.146.197
10.10.146.196
10.10.146.182
10.10.146.172
10.10.146.193
10.10.146.168
10.10.146.184
10.10.146.171
10.10.146.175
10.10.146.181
10.10.146.195
10.10.146.178
10.10.146.186
10.10.146.183
10.10.146.190
10.10.146.179
10.10.146.173
10.10.146.177
10.10.146.185
10.10.146.194
10.10.146.174
10.10.146.180
10.10.146.176
10.10.146.187
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
       --attribute-names "BootMode,PxeDev1EnDis,CpuMinSevAsid,LogicalProc,NumaNodesPerSocket,PcieEnhancedPreferredIo,PciePreferredIoBus,ProcVirtualization,SetBootOrderDis,SetBootOrderEn,OneTimeUefiBootSeqDev,OneTimeBootMode,SriovGlobalEnable,ErrPrompt" \
       --attribute-values "Uefi,Disabled,253,Enabled,0,Enabled,Enabled,Enabled,Disk.Bay.9:Enclosure.Internal.0-1,Floppy.iDRACVirtual.1-1,Floppy.iDRACVirtual.1-1,Enabled,Enabled,Disabled" --duration-time 1200 --reboot &
done
wait

# Get BIOS configuration for the nodes. This is a lot of data, and you likely do not have to run it.
date
for IP in "${IPS[@]}"; do
    python3 ./GetSetBiosAttributesREDFISH.py -ip $IP -u $USER -p $PASSWORD \
        --get-attribute PxeDev1EnDis &
done
wait

date
for IP in "${IPS[@]}"; do
    python3 ./GetSetBiosAttributesREDFISH.py -ip $IP -u $USER -p $PASSWORD \
        --get-attribute SetBootOrderEn &
done
wait

date
for IP in "${IPS[@]}"; do
    python3 ./GetSetBiosAttributesREDFISH.py -ip $IP -u $USER -p $PASSWORD \
        --get-attribute OneTimeUefiBootSeqDev &
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
GIT_REV=a17247bd86c7aa4e87742bf74d108614580f216d
curl -LO https://download.dfinity.systems/ic/$GIT_REV/setup-os/disk-img/disk-img.tar.gz
tar -xvf disk-img.tar.gz

cd $HOME
LOOPDEV=/dev/loop3001
sudo losetup --partscan --show ${LOOPDEV} disk.img
mkdir -p /tmp/mounts/setupos-config
sudo mount ${LOOPDEV}p3 /tmp/mounts/setupos-config

# update config at /tmp/mounts/setupos-config
mkdir -p $HOME/setupos-config-bo1
cat >| $HOME/setupos-config-bo1/config.ini <<_EOF
# Please update the template/example below.
#
# If you need help, please do not hesitate to contact the
# Internet Computer Association.
#
ipv6_prefix=2600:c0d:3002:4
ipv6_subnet=/64
ipv6_gateway=2600:c0d:3002:4::1
_EOF

# !!!!!!!!!!!!!!!
# !!!IMPORTANT!!!
# !!!!!!!!!!!!!!!
# download the latest version of $HOME/setupos-config-bo1/ssh_authorized_keys/admin from
# https://gitlab.com/dfinity-lab/private/infrasec/prod-ssh/-/blob/main/mercury/admin
# as instructed in the BMC repo https://gitlab.com/dfinity-lab/teams/node-team/bmc-remote-deployment/-/blob/master/README.md
mkdir -p $HOME/setupos-config-bo1/ssh_authorized_keys/
vi $HOME/setupos-config-bo1/ssh_authorized_keys/admin

# !!!!!!!!!!!!!!!
# !!!IMPORTANT!!!
# !!!!!!!!!!!!!!!
# download the latest version of $HOME/setupos-config-bo1/node_operator_private_key.pem from 1password
sudo cp $HOME/setupos-config-bo1/config.ini /tmp/mounts/setupos-config/config.ini
sudo cp $HOME/setupos-config-bo1/ssh_authorized_keys/admin /tmp/mounts/setupos-config/ssh_authorized_keys/admin
sudo cp $HOME/setupos-config-bo1/node_operator_private_key.pem /tmp/mounts/setupos-config/node_operator_private_key.pem
# Unmount and proceed
sudo umount /tmp/mounts/setupos-config
sudo losetup -d ${LOOPDEV}
sudo cp $HOME/disk.img /var/www/html/bo1-setupos.img
sudo chmod 0644 /var/www/html/bo1-setupos.img
```

Create a list of nodes to be deployed:
```bash
cat >| $HOME/bmc-remote-deployment/deploy-bo1.csv <<_EOF
10.10.146.192,$USER,$PASSWORD
10.10.146.191,$USER,$PASSWORD
10.10.146.188,$USER,$PASSWORD
10.10.146.189,$USER,$PASSWORD
10.10.146.197,$USER,$PASSWORD
10.10.146.196,$USER,$PASSWORD
10.10.146.182,$USER,$PASSWORD
10.10.146.172,$USER,$PASSWORD
10.10.146.193,$USER,$PASSWORD
10.10.146.168,$USER,$PASSWORD
10.10.146.184,$USER,$PASSWORD
10.10.146.171,$USER,$PASSWORD
10.10.146.175,$USER,$PASSWORD
10.10.146.181,$USER,$PASSWORD
10.10.146.195,$USER,$PASSWORD
10.10.146.178,$USER,$PASSWORD
10.10.146.186,$USER,$PASSWORD
10.10.146.183,$USER,$PASSWORD
10.10.146.190,$USER,$PASSWORD
10.10.146.179,$USER,$PASSWORD
10.10.146.173,$USER,$PASSWORD
10.10.146.177,$USER,$PASSWORD
10.10.146.185,$USER,$PASSWORD
10.10.146.194,$USER,$PASSWORD
10.10.146.174,$USER,$PASSWORD
10.10.146.180,$USER,$PASSWORD
10.10.146.176,$USER,$PASSWORD
10.10.146.187,$USER,$PASSWORD
_EOF
head -n1 $HOME/bmc-remote-deployment/deploy-bo1.csv > deploy-bo1-node-1.csv
```

Serve the image with http (nginx) and deploy node(s):
```bash
cd $HOME/bmc-remote-deployment
IP_ADDR=$(ip a s | grep "inet " | grep "scope global" | head -n1 | awk '{print $2}' | cut -d/ -f1)
poetry run ./deploy.py --csv-filename deploy-bo1-node-1.csv --network-image-url http://$IP_ADDR/bo1-setupos.img --wait-time 20
```

Once the first node is deployed, you can use the CSV file for the other nodes. For instance, `deploy-bo1-node-2.csv`, or for multiple nodes, say `deploy-bo1-node-7+.csv`.

After all nodes are deployed, make sure you turn off nginx, since it serves an image with the node operator private key embedded.

```
sudo systemctl stop nginx
sudo systemctl disable nginx
```

Now you need to inject the admin ssh keys into the GuestOS.
You can get the GuestOS IPs from the internal dashboard or from FactsDB.
```bash
HOST_IPv6=(
2600:c0d:3002:4:6800:21ff:febb:aa9a
2600:c0d:3002:4:6800:e0ff:fe3e:cd28
2600:c0d:3002:4:6800:95ff:fe9b:81c3
2600:c0d:3002:4:6800:c1ff:fea1:e37b
2600:c0d:3002:4:6800:89ff:fe7c:5cba
2600:c0d:3002:4:6800:94ff:fec9:6b
2600:c0d:3002:4:6800:35ff:feb2:86e9
2600:c0d:3002:4:6800:ccff:feca:b74a
2600:c0d:3002:4:6800:82ff:fec9:49d5
2600:c0d:3002:4:6800:bcff:fea0:dea1
2600:c0d:3002:4:6800:19ff:fe8c:de47
2600:c0d:3002:4:6800:66ff:fef6:eb05
2600:c0d:3002:4:6800:e8ff:fe63:e52b
2600:c0d:3002:4:6800:5bff:fe9c:57c6
2600:c0d:3002:4:6800:46ff:fe78:ef9e
2600:c0d:3002:4:6800:45ff:fe34:dbe7
2600:c0d:3002:4:6800:f2ff:feb4:de96
2600:c0d:3002:4:6800:d3ff:fe3a:c18e
2600:c0d:3002:4:6800:18ff:fe27:db36
2600:c0d:3002:4:6800:1ff:fe38:393a
2600:c0d:3002:4:6800:3eff:fe99:d2
2600:c0d:3002:4:6800:dfff:fee7:5ae8
2600:c0d:3002:4:6800:bcff:fee7:4008
2600:c0d:3002:4:6800:e7ff:fe29:db0e
2600:c0d:3002:4:6800:f8ff:fe30:b32f
2600:c0d:3002:4:6800:59ff:fe2a:f53b
2600:c0d:3002:4:6800:5ff:fe3a:7089  # Not yet deployed
2600:c0d:3002:4:6800:9eff:fe4d:57c5 # Not yet deployed
)

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
