import base64
import csv
import json
import logging
import pathlib
import re
import subprocess
import typing
from multiprocessing import Pool

import peewee
import yaml

from pylib import ic_admin
from pylib import ic_deployment
from pylib import ic_utils

# Keep data in an in-memory database during runtime
db = peewee.SqliteDatabase(":memory:")
ic_adm: typing.Optional[ic_admin.IcAdmin] = None  # Will be set from main.py
deployment: typing.Optional[ic_deployment.IcDeployment] = None  # Will be set from main.py
dump_fields: typing.List = []  # Will be set from main.py
dump_filter: str = ""  # Will be set from main.py


class FileStorage:
    """Interface to the CSV files in which the data is stored."""

    def __init__(self):
        """Create a FileStorage object."""
        self.deployment_name = deployment.name
        self.base_dir = pathlib.Path(__file__).parent / "data"
        self.base_dir.mkdir(exist_ok=True)

    @property
    def physical_csv(self):
        """Return the file path in which the physical system data is stored."""
        return self.base_dir / f"{self.deployment_name}_physical_systems.csv"

    @property
    def guests_csv(self):
        """Return the file path in which the node (guest) data is stored."""
        return self.base_dir / f"{self.deployment_name}_guests.csv"

    @property
    def principals_csv(self):
        """Return the file path in which the principals data is stored."""
        return self.base_dir / f"{self.deployment_name}_principals.csv"

    @property
    def subnets_csv(self):
        """Return the file path in which the subnets data is stored."""
        return self.base_dir / f"{self.deployment_name}_subnets.csv"


# pylint: disable=no-member
class FactsTable(peewee.Model):
    """General methods shared by all Facts tables."""

    @classmethod
    def add_row(cls, kwargs):
        """Create a row in the table or raise an IntegrityError."""
        with db.atomic():
            cls.create(**kwargs)

    @classmethod
    def dump(cls, out: typing.TextIO):
        """Dump values to the provide out file, one line per row."""
        filter_field, filter_value = None, None
        if dump_filter and "=" in dump_filter:
            filter_field, filter_value = dump_filter.split("=")
        for row in cls.select():
            if filter_field and filter_value:
                value = getattr(row, filter_field, "")
                if not re.match(filter_value, value):
                    continue
            line = str(row)
            if not line:
                continue
            out.write(line + "\n")

    @classmethod
    def to_dict(cls):
        """Return the table as a list of dicts."""
        result = []
        for row in cls.select():
            row_dict = {}
            for field in row._meta.fields.keys():  # pylint: disable=protected-access
                value = getattr(row, field)
                if value:
                    row_dict[field] = value
            result.append(row_dict)
        return result

    @classmethod
    def set_values(cls, name: str, **kwargs):
        """For the given name, set the values of the provided arguments."""
        query = cls.update(kwargs).where(cls.name == name)
        query.execute()

    @classmethod
    def clear(cls):
        """Delete all rows in the table."""
        query = cls.delete()
        query.execute()  # pylint: disable=no-value-for-parameter

    def __str__(self):
        """Return the string representation of the object, compatible with the Ansible ini file format."""
        res = []
        if dump_fields and len(dump_fields) == 1:
            field = dump_fields[0]
            value = getattr(self, field, "")
            if value:
                res = [value]
        else:
            res = [self.name]
            for field in self._meta.fields.keys():
                if field == "name":
                    continue
                if not dump_fields or field in dump_fields:
                    value = getattr(self, field)
                    if value:
                        res.append(f"{field}={value}")
        return " ".join(res)

    class Meta:
        """Required by peewee."""

        database = db


def load_repo_serial_numbers():
    serial_numbers_filename = ic_utils.repo_root / "deployments/env/serial-numbers.yml"
    return yaml.load(open(serial_numbers_filename, encoding="utf8"), Loader=yaml.FullLoader)


class PhysicalSystem(FactsTable):
    """Physical system facts."""

    name = peewee.CharField(primary_key=True, unique=True, index=True)
    serial_number = peewee.CharField(null=True, unique=True, index=True)
    hsm_pubkey = peewee.CharField(null=True)

    @classmethod
    def refresh(cls, refresh_hsm_public_keys: bool = False):
        """Refresh (try to fill in) the facts for all physical systems."""
        hsm_pubkey_query_hosts = []
        repo_serial_numbers = load_repo_serial_numbers()
        base_path = ic_utils.repo_root / "deployments/env" / deployment.name / "data_centers"

        for name in deployment.get_deployment_physical_hosts():
            record = cls.get_or_create(name=name)[0]
            if not record.serial_number:
                if name in repo_serial_numbers:
                    record.serial_number = repo_serial_numbers[name]
            if not record.serial_number:
                (ret_code, stdout, stderr) = ic_utils.ssh_run_command(
                    record.name,
                    username=ic_utils.PHY_HOST_USER,
                    command="sudo dmidecode -s system-serial-number",
                    do_not_raise=False,
                    binary_stdout=False,
                )
                if ret_code != 0:
                    logging.error("Getting the serial number failed on %s: %s", record.name, stderr)
                    continue
                record.serial_number = stdout.strip()
            if not record.serial_number:
                logging.error("PhysicalSystem %s does not have a serial number, skipping.")
                continue
            if not record.hsm_pubkey and refresh_hsm_public_keys:
                hsm_pubkey_desc = "dc_public_key_" + name.split(".", 1)[0]
                hsm_pubkey_filename = hsm_pubkey_desc + ".der"
                if (base_path / hsm_pubkey_filename).exists():
                    record.hsm_pubkey = base64.b64encode(open(base_path / hsm_pubkey_filename, "rb").read())
                else:
                    hsm_pubkey_query_hosts.append(name)
            record.save()

        if hsm_pubkey_query_hosts:
            logging.info(
                "PhysicalSystem records to query the HSM public key: %s",
                hsm_pubkey_query_hosts,
            )
        for host, (ret_code, stdout, stderr) in zip(
            hsm_pubkey_query_hosts,
            ic_utils.parallel_ssh_run(
                hsm_pubkey_query_hosts,
                username=None,
                command="sudo apt install -qqy opensc >/dev/null 2>&1; pkcs11-tool -r --slot 0x0 -y pubkey -d 01",
                binary_stdout=True,
            ),
        ):
            if ret_code != 0:
                logging.error("Getting the HSM public key failed on %s: %s", host, stderr)
                continue
            record = cls.get_or_create(name=host)[0]
            record.hsm_pubkey = base64.b64encode(stdout).decode("utf8")
            record.save()

    @classmethod
    def update_repo_serial_numbers_yml(cls):
        """Update the serial numbers shared by the Ansible testnet deployments."""
        serial_numbers_filename = ic_utils.repo_root / "deployments/env/serial-numbers.yml"
        serial_numbers = yaml.load(open(serial_numbers_filename, encoding="utf8"), Loader=yaml.FullLoader)
        for row in cls.select():
            serial_numbers[row.name] = row.serial_number
        with open(serial_numbers_filename, "w", encoding="utf8") as f:
            yaml.dump(serial_numbers, f)

    @classmethod
    def update_repo_hsm_public_keys(cls):
        """Update the HSM public keys necessary for the HSM-based authentication in deployments."""
        base_path = ic_utils.repo_root / "deployments/env" / deployment.name / "data_centers"
        meta_json_path = base_path / "meta.json"
        meta_json = json.load(open(meta_json_path, encoding="utf8"))
        for row in cls.select():
            hsm_pubkey_desc = "dc_public_key_" + row.name.split(".", 1)[0]
            hsm_pubkey_filename = hsm_pubkey_desc + ".der"
            hsm_pubkey_path = base_path / hsm_pubkey_filename
            with open(hsm_pubkey_path, "wb") as f:
                hsm_pubkey = base64.b64decode(row.hsm_pubkey)
                f.write(hsm_pubkey)
            meta_json[hsm_pubkey_desc] = {
                "node_allowance": 64,
                "node_provider": hsm_pubkey_filename,
            }
            principal_id_path: pathlib.PosixPath = base_path / (hsm_pubkey_desc + ".principal")
            if not principal_id_path.exists() or principal_id_path.stat().st_size == 0:
                principal = (
                    subprocess.check_output(["release_cli", "der-to-principal", hsm_pubkey_path]).decode("utf8").strip()
                )
                with open(principal_id_path, "w", encoding="utf8") as f:
                    f.write(principal)

        with open(meta_json_path, "w", encoding="utf8") as f:
            f.write(json.dumps(meta_json, indent=2))
        logging.info("Updated the HSM public keys in %s", base_path)


class Guest(FactsTable):
    """Guest (Deployment Node) facts table."""

    name = peewee.CharField(primary_key=True, unique=True, index=True)
    node_type = peewee.CharField(null=True)
    ipv6 = peewee.CharField(null=True)
    principal = peewee.CharField(null=True)
    subnet = peewee.CharField(null=True)
    physical_system = peewee.ForeignKeyField(PhysicalSystem, backref="guests", null=True)

    @classmethod
    def refresh(cls):
        """Refresh (fill in missing fields) in the table."""
        logging.info("Refreshing Guests for deployment: %s", deployment.name)
        for node, hostvars in deployment.get_deployment_nodes_hostvars().items():
            ic_host = hostvars["ic_host"]
            physical_system = PhysicalSystem.get(name=f"{ic_host}.{ic_host[:3]}.dfinity.network")
            name = node
            record = cls.get_or_create(name=name)[0]
            record.node_type = hostvars.get("node_type", "")
            record.ipv6 = hostvars["ipv6_address"]
            record.principal = Principal.get_principal_for_ipv6(hostvars["ipv6_address"])
            record.subnet = Principal.get_subnet_for_principal(record.principal)
            record.physical_system = physical_system
            record.save()

    @classmethod
    def get_principal_for_physical_system(cls, needle: str):
        """Return the principal for the provided host string."""
        try:
            guests = (
                Guest.select(Guest, PhysicalSystem).join(PhysicalSystem).where(PhysicalSystem.name ** f"%{needle}%")
            )
            if guests.count() < 1:
                logging.info("No guests found on the host %s", needle)
                return ""
            elif guests.count() == 1:
                return guests.get().principal
            else:
                res = []
                for guest in guests:
                    res.append(guest.name)
                logging.info("Multiple guests founds for the input %s: %s", needle, ", ".join(res))
                return ""
        except cls.DoesNotExist:
            return ""
        return ""

    @classmethod
    def generate_ansible_inventory(cls):
        """Check if the entries in the Ansible inventory are correct, and print any discrepancies."""
        subnets = {}
        for s in Subnet.select().dicts():
            subnets[s["name"]] = dict(s)

        with open(ic_utils.repo_root / f"deployments/env/{deployment.name}/hosts.ini", encoding="utf8") as hosts_ini:
            hosts_ini = hosts_ini.readlines()
            for row in cls.select().dicts():
                subnet_row = subnets[row["subnet"] or "unassigned"]
                subnet_num = subnet_row["number"] or "unassigned"
                if subnet_num == "0":
                    subnet_row["ansible_group"] = "nns"
                else:
                    subnet_row["ansible_group"] = "subnet_%s" % subnet_num
                row["subnet_id"] = subnets[row["subnet"] or "unassigned"]["name"]
                row["hosts_ini_line"] = [x for x in hosts_ini if row["name"] in x][0]
                subnet_guests = subnet_row.get("guests", [])
                subnet_guests.append(row)
                subnet_row["guests"] = subnet_guests

        for subnet in subnets.values():
            # print(subnet)
            print("\n[%s]  # %s" % (subnet["ansible_group"], subnet["name"]))
            for guest in subnet["guests"]:
                print(guest["hosts_ini_line"].strip())


class Subnet(FactsTable):
    """Subnet IDs for the deployment."""

    name = peewee.CharField(primary_key=True, unique=True, index=True)
    number = peewee.IntegerField(null=True)
    replica_version = peewee.CharField(null=True)

    @classmethod
    def refresh(cls):
        """Refresh (fill in missing fields) in the table."""
        logging.info("Refreshing Subnets for deployment: %s", deployment.name)
        subnet_replica_versions = ic_adm.get_subnet_replica_versions()
        for i, (name, _subnet) in enumerate(ic_adm.get_subnets().items()):
            # get_or_create Returns: Tuple of Model instance and boolean indicating if a new object was created.
            record = cls.get_or_create(name=name)[0]
            record.number = i
            record.replica_version = subnet_replica_versions.get(name)
            record.save()
        record = cls.get_or_create(name="unassigned")[0]
        record.number = None
        record.replica_version = None
        record.save()

    def __str__(self):
        """Return the string representation of the object, compatible with the Ansible ini file format."""
        res = [self.name]
        for field in self._meta.fields.keys():
            if field == "name":
                continue
            value = getattr(self, field)
            if value:
                res.append(f"{field}={value}")
        members = []
        for member in self.members:
            try:
                physical_system_short = Guest.get(principal=member).physical_system.name.split(".")[0]
                members.append(physical_system_short)
            except peewee.DoesNotExist:
                members.append("ERROR:%s" % member.name)
        res.append(f"members_len={len(members)}")
        res.append(f"members={','.join(members)}")
        return " ".join(res)


class Principal(FactsTable):
    """Principal IDs for the deployment nodes."""

    name = peewee.CharField(primary_key=True, unique=True, index=True)
    subnet = peewee.ForeignKeyField(Subnet, backref="members", null=True)
    ipv6 = peewee.CharField(null=True)

    @classmethod
    def refresh(cls):
        """Refresh (fill in missing fields) in the table."""
        logging.info("Refreshing Principals for deployment: %s", deployment.name)

        nodes_unknown_ipv6 = []
        for name, subnet in ic_adm.get_node_ids().items():
            # get_or_create Returns: Tuple of Model instance and boolean indicating if a new object was created.
            record = cls.get_or_create(name=name)[0]
            if subnet:
                record.subnet = Subnet.get(name=subnet)
                logging.debug("Updating subnet for record: %s", record)
            if not record.ipv6:
                nodes_unknown_ipv6.append(name)
            record.save()

        if nodes_unknown_ipv6:
            with Pool(32) as p:
                nodes_ipv6 = p.map(ic_adm.node_get_ipv6, nodes_unknown_ipv6)
                for name, ipv6 in zip(nodes_unknown_ipv6, nodes_ipv6):
                    record = cls.get_or_create(name=name)[0]
                    record.ipv6 = ipv6
                    logging.debug("Updating ipv6 for record: %s", record)
                    record.save()

    @classmethod
    def get_principal_for_ipv6(cls, ipv6: str):
        """Return the principal for the provided IPv6 address."""
        try:
            record = cls.get(cls.ipv6 == ipv6)
            return record.name
        except cls.DoesNotExist:
            return

    @classmethod
    def get_subnet_for_principal(cls, name: str):
        """Return the subnet for the provided principal."""
        try:
            record = cls.get(cls.name == name)
            return record.subnet
        except cls.DoesNotExist:
            return

    def __str__(self):
        """Return the string representation of the object, compatible with the Ansible ini file format."""
        res = [self.name]
        for field in self._meta.fields.keys():
            if field == "name":
                continue
            value = getattr(self, field)
            if value:
                if field == "subnet":
                    res.append(f"{field}={value.name}")
                else:
                    res.append(f"{field}={value}")
        return " ".join(res)


def table_load_from_csv(table, csv_path):
    """Load table contents from a CSV file."""
    logging.debug("Loading data for the table %s from file %s.", table, csv_path)
    if not csv_path.exists():
        return
    with open(csv_path, "r", encoding="utf8") as f_read:
        reader = csv.DictReader(f_read)
        for row in reader:
            table.get_or_create(**row)


def table_save_to_csv(table, csv_path):
    """Save the table contents to a CSV file."""
    if len(table.select()) == 0:
        logging.debug("Table %s is empty, will not save.", table)
        return
    logging.debug("Saving data for the table %s to the file %s.", table, csv_path)
    with open(csv_path, "w", encoding="utf8") as f_out:
        # pylint: disable=protected-access
        writer = csv.DictWriter(f_out, table._meta.fields.keys(), lineterminator="\n")
        writer.writeheader()
        for row in table.select().dicts():
            writer.writerow(row)


def db_open_and_load(file_storage: FileStorage):
    """At startup, load the database tables contents from the CSV files."""
    db.connect()
    db.create_tables([PhysicalSystem, Guest, Principal, Subnet])
    logging.debug("Connecting and loading the database for deployment %s.", deployment.name)
    table_load_from_csv(PhysicalSystem, file_storage.physical_csv)
    table_load_from_csv(Guest, file_storage.guests_csv)
    table_load_from_csv(Principal, file_storage.principals_csv)
    table_load_from_csv(Subnet, file_storage.subnets_csv)


def db_save_and_close(file_storage):
    """On shutdown, save the in-memory database contents to the CSV files."""
    logging.debug("Saving the database for %s deployment.", deployment.name)
    table_save_to_csv(PhysicalSystem, file_storage.physical_csv)
    table_save_to_csv(Guest, file_storage.guests_csv)
    table_save_to_csv(Principal, file_storage.principals_csv)
    table_save_to_csv(Subnet, file_storage.subnets_csv)
