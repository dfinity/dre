
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

Not strictly necessary but may come handy if we want to run nginx in a container.
Podman: guide available at https://www.cyberithub.com/how-to-install-podman-on-ubuntu-20-04-lts-step-by-step/
```bash
source /etc/os-release
echo "deb https://download.opensuse.org/repositories/devel:/kubic:/libcontainers:/stable/xUbuntu_${VERSION_ID}/ /" | sudo tee /etc/apt/sources.list.d/devel:kubic:libcontainers:stable.list
curl -L https://download.opensuse.org/repositories/devel:/kubic:/libcontainers:/stable/xUbuntu_${VERSION_ID}/Release.key | sudo apt-key add -
apt-get update
apt-get -y upgrade
podman --version
```

# Cloning the repo with the scripts for the remote deployment

```bash
cd $HOME
git clone git@gitlab.com:dfinity-lab/teams/node-team/bmc-remote-deployment.git
```

# Prepare the machines - BIOS settings and firmware

Firmware needs to be updated to the latest version and PXE boot needs to be disabled to prevent machines from booting to MaaS.

```bash
IPS=(
10.10.151.195
10.10.151.198
10.10.151.172
10.10.151.184
10.10.151.196
10.10.151.186
10.10.151.182
10.10.151.168
10.10.151.179
10.10.151.189
10.10.151.192
10.10.151.194
10.10.151.191
10.10.151.178
10.10.151.180
10.10.151.171
10.10.151.177
10.10.151.174
10.10.151.181
10.10.151.173
10.10.151.193
10.10.151.190
10.10.151.187
10.10.151.169
10.10.151.175
10.10.151.183
10.10.151.185
10.10.151.176
)
USER=root
PASSWORD=calvin

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

# Get BIOS configuration for the nodes. This is a lot of data, and you likely do not have to run it.
date
for IP in "${IPS[@]}"; do
    python3 ./GetSetBiosAttributesREDFISH.py -ip $IP -u $USER -p $PASSWORD &
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
       --attribute-names "BootMode,PxeDev1EnDis,CpuMinSevAsid,LogicalProc,NumaNodesPerSocket,PcieEnhancedPreferredIo,PciePreferredIoBus,ProcVirtualization,SetBootOrderEn,SriovGlobalEnable,ErrPrompt" \
       --attribute-values "Uefi,Disabled,253,Enabled,0,Enabled,Enabled,Enabled,Floppy.iDRACVirtual.1-1,Enabled,Disabled" --reboot &
done
wait

# Power on all machines.
date
for IP in "${IPS[@]}"; do
    python3 ./GetSetPowerStateREDFISH.py -ip $IP -u $USER -p $PASSWORD --set On &
done
```

# Prepare the SetupOS image

Instructions at https://gitlab.com/dfinity-lab/teams/node-team/bmc-remote-deployment
```bash
cd $HOME
GIT_REV=d6d395a480cd6986b4788f4aafffc5c03a07e46e
curl -LO https://download.dfinity.systems/ic/$GIT_REV/setup-os/disk-img/disk-img.tar.gz
tar -xvf disk-img.tar.gz

LOOPDEV=/dev/loop3001
sudo losetup --partscan --show ${LOOPDEV} disk.img
mkdir -p /tmp/mounts/setupos-config
sudo mount ${LOOPDEV}p3 /tmp/mounts/setupos-config

# update config at /tmp/mounts/setupos-config
mkdir -p $HOME/setupos-config-sh1
cat >| $HOME/setupos-config-sh1/config.ini <<_EOF
# Please update the template/example below.
#
# If you need help, please do not hesitate to contact the
# Internet Computer Association.
#
ipv6_prefix=2001:4c08:2003:b09
ipv6_subnet=/64
ipv6_gateway=2001:4c08:2003:b09::1
_EOF

# !!!!!!!!!!!!!!!
# !!!IMPORTANT!!!
# !!!!!!!!!!!!!!!
# download the latest version of $HOME/setupos-config-sh1/ssh_authorized_keys/admin from
# https://gitlab.com/dfinity-lab/private/infrasec/prod-ssh/-/blob/main/mercury/admin
# as instructed in the BMC repo https://gitlab.com/dfinity-lab/teams/node-team/bmc-remote-deployment/-/blob/master/README.md

# !!!!!!!!!!!!!!!
# !!!IMPORTANT!!!
# !!!!!!!!!!!!!!!
# download the latest version of $HOME/setupos-config-sh1/node_operator_private_key.pem from 1password
sudo cp $HOME/setupos-config-sh1/config.ini /tmp/mounts/setupos-config/config.ini
sudo cp $HOME/setupos-config-sh1/ssh_authorized_keys/admin /tmp/mounts/setupos-config/ssh_authorized_keys/admin
sudo cp $HOME/setupos-config-sh1/node_operator_private_key.pem /tmp/mounts/setupos-config/node_operator_private_key.pem
# Unmount and proceed
sudo umount /tmp/mounts/setupos-config
sudo losetup -d ${LOOPDEV}
```

Serve the image with http (nginx):
```bash
sudo cp disk.img /var/www/html/sh1-setupos.img
cd bmc-remote-deployment
IP_ADDR=$(ip a s | grep "inet " | grep "scope global" | head -n1 | awk '{print $2}' | cut -d/ -f1)
poetry run ./deploy.py --csv-filename deploy-sh1-node-1.csv --network-image-url http://$IP_ADDR/sh1-setupos.img --wait-time 20
```

Once the first node is deployed, you can use the CSV file for the other nodes. For instance, `deploy-sh1-node-2.csv`, or for multiple nodes, say `deploy-sh1-node-7+.csv`.

After all nodes are deployed, make sure you turn off nginx, since it serves an image with the node operator private key embedded.

```
sudo systemctl stop nginx
sudo systemctl disable nginx
```

Now you need to inject the admin ssh keys into the GuestOS.
You can get the GuestOS IPs from the internal dashboard or from FactsDB.
```bash
IPS=(
2001:4c08:2003:b09:6800:f5ff:fe5b:d6a4
2001:4c08:2003:b09:6800:f6ff:fee8:4870
2001:4c08:2003:b09:6800:2aff:fe23:e9a0
2001:4c08:2003:b09:6800:83ff:fee3:3612
2001:4c08:2003:b09:6800:f2ff:febd:7436
2001:4c08:2003:b09:6800:2aff:fe96:4ede
2001:4c08:2003:b09:6800:2ff:fe8b:9acb
2001:4c08:2003:b09:6800:59ff:fea7:c5ba
2001:4c08:2003:b09:6800:6dff:fe97:2edb
2001:4c08:2003:b09:6800:1ff:fe2a:bbdb
2001:4c08:2003:b09:6800:92ff:fe52:2c52
2001:4c08:2003:b09:6800:46ff:fe70:d6e0
2001:4c08:2003:b09:6800:8fff:fed3:39ff
2001:4c08:2003:b09:6800:16ff:fe49:afad
2001:4c08:2003:b09:6800:c4ff:feeb:c02b
2001:4c08:2003:b09:6800:baff:fe41:fa2c
2001:4c08:2003:b09:6800:ffff:feee:b46a
2001:4c08:2003:b09:6800:9bff:fe95:94a3
2001:4c08:2003:b09:6800:14ff:fe8d:5e38
2001:4c08:2003:b09:6800:59ff:febd:d67a
2001:4c08:2003:b09:6800:48ff:fece:5bb1
2001:4c08:2003:b09:6800:bfff:fedb:44e0
2001:4c08:2003:b09:6800:ebff:fe49:798e
2001:4c08:2003:b09:6800:a3ff:fe5a:51f
2001:4c08:2003:b09:6800:bbff:fef0:c58e
2001:4c08:2003:b09:6800:e5ff:fe72:cdad
2001:4c08:2003:b09:6800:78ff:fe2e:c8fe
2001:4c08:2003:b09:6800:3fff:fe39:cb3d
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

for IP in "${IPS[@]}"; do
    set -x
    scp -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -6 inject-admin.sh admin@"[$IP]":
    ssh -tt -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -6 admin@$IP "chmod +x inject-admin.sh && sudo ./inject-admin.sh"
    set +x
done
```
