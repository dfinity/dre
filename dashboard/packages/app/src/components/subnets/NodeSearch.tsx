import React, { forwardRef } from 'react';
import { Link, makeStyles } from '@material-ui/core';
import { Table, TableColumn } from '@backstage/core-components';
import { fetchMissingGuests, fetchNodes, fetchNodesHealths, fetchSubnets } from './fetch';
import { NodeHealth, Node, Subnet, Operator } from './types';
import SearchIcon from '@material-ui/icons/Search';

// table icon imports
import ChevronLeftIcon from '@material-ui/icons/ChevronLeft';
import ChevronRight from '@material-ui/icons/ChevronRight';
import AddBox from '@material-ui/icons/AddBox';
import ArrowUpward from '@material-ui/icons/ArrowUpward';
import Check from '@material-ui/icons/Check';
import Clear from '@material-ui/icons/Clear';
import DeleteOutline from '@material-ui/icons/DeleteOutline';
import Edit from '@material-ui/icons/Edit';
import FilterList from '@material-ui/icons/FilterList';
import FirstPage from '@material-ui/icons/FirstPage';
import LastPage from '@material-ui/icons/LastPage';
import Remove from '@material-ui/icons/Remove';
import SaveAlt from '@material-ui/icons/SaveAlt';
import ViewColumn from '@material-ui/icons/ViewColumn';


const useStyles = makeStyles(theme => ({
    container: {
        width: "100%",
        '& table': {
            display: "none",
        }
    },
    containerExpanded: {
        width: "100%",
        '& td, th': {
            whiteSpace: "nowrap",
            // Shrink the cell to fit text
            // width: "0.1% !important",
        }
    },
    empty: {
        padding: theme.spacing(2),
        display: 'flex',
        justifyContent: 'center',
    },
}));

export default function NodeSearch({ onSearchChange, expand }: { onSearchChange: (_: string) => void, expand: boolean }) {

    const classes = useStyles();
    const columns: TableColumn<Node & { health?: NodeHealth, subnetDetailed?: Subnet, dfinity_owned: boolean }>[] = [
        {
            title: 'Principal',
            emptyValue: "",
            field: 'principal',
            type: 'string',
        },
        {
            title: 'Label',
            emptyValue: "",
            field: 'label',
            type: 'string',
        },
        {
            title: 'IP',
            emptyValue: "",
            field: 'ip_addr',
        },
        {
            title: 'Health',
            emptyValue: "",
            field: 'health',
        },
        {
            title: 'Subnet',
            emptyValue: "",
            field: 'subnetDetailed.metadata.name',
            type: 'string',
            render: (rowData) => rowData.subnet && `${rowData.subnetDetailed?.metadata.name} (${rowData.subnet})`
        },
        {
            field: 'subnet',
            type: 'string',
            hidden: true,
            searchable: true,
        },
        {
            title: 'Provider',
            emptyValue: "",
            field: 'operator.provider.name',
            render: (rowData) => {
                const display = `${rowData.operator.provider.name ?? "Unknown"} (${rowData.operator.provider.principal.split("-")[0]})`;
                const link = (
                    <Link target="_blank" href={rowData.operator.provider.website}>
                        {display}
                    </Link>
                );
                return rowData.operator.provider.website ? link : display
            }
        },
        {
            field: 'operator.provider.principal',
            type: 'string',
            hidden: true,
            searchable: true,
        },
        {
            title: 'DC',
            emptyValue: "",
            field: 'operator.datacenter.name',
            hidden: true,
            searchable: true,
        },
        {
            title: 'DC owner',
            emptyValue: "",
            field: 'operator.datacenter.owner.name',
        },
        {
            title: 'City',
            emptyValue: "",
            field: 'operator.datacenter.city',
        },
        {
            title: 'Country',
            emptyValue: "",
            field: 'operator.datacenter.country',
        },
        {
            title: 'Continent',
            emptyValue: "",
            field: 'operator.datacenter.continent',
        },
        {
            title: 'Admin',
            field: 'dfinity_owned',
            type: 'boolean',
            hidden: false,
            searchable: true,
        },
        {
            title: 'Decentralized',
            field: 'decentralized',
            type: 'boolean',
            hidden: false,
        },
    ];


    const healths = fetchNodesHealths();
    const subnets = Object.values(fetchSubnets());
    const nodes = [...Object.values(fetchNodes()), ...fetchMissingGuests().map(g => ({
        principal: "",
        ip_addr: g.ipv6,
        operator: {} as Operator,
        label: g.name,
        dfinity_owned: g.dfinity_owned,
    } as Node))].map(n => ({
        subnetDetailed: subnets.find(s => s.nodes.find(sn => sn.principal == n.principal)),
        health: healths[n.principal] ?? "",
        ...n
    }));

    return (
        <div className={expand ? classes.containerExpanded : classes.container}>
            <Table
                options={{
                    padding: 'dense',
                    emptyRowsWhenPaging: false,
                    pageSize: 15,
                    pageSizeOptions: [],
                    search: true,
                    searchFieldAlignment: 'left',
                    searchAutoFocus: true,
                    searchFieldVariant: 'outlined',
                    searchFieldStyle: {
                        fontSize: '1.1rem',
                        minWidth: '600px',
                    },
                }}
                localization={{
                    toolbar: {
                        searchPlaceholder: 'Search for nodes by IP, hostname, subnet, etc. ...',
                    }
                }}
                // Icons sourced from backstage source code since there's no nice way to override individual icons
                icons={{
                    Add: forwardRef((props, ref) => <AddBox {...props} ref={ref} />),
                    Check: forwardRef((props, ref) => <Check {...props} ref={ref} />),
                    Clear: forwardRef((props, ref) => <Clear {...props} ref={ref} />),
                    Delete: forwardRef((props, ref) => <DeleteOutline {...props} ref={ref} />),
                    DetailPanel: forwardRef((props, ref) => <ChevronRight {...props} ref={ref} />),
                    Edit: forwardRef((props, ref) => <Edit {...props} ref={ref} />),
                    Export: forwardRef((props, ref) => <SaveAlt {...props} ref={ref} />),
                    Filter: forwardRef((props, ref) => <FilterList {...props} ref={ref} />),
                    FirstPage: forwardRef((props, ref) => <FirstPage {...props} ref={ref} />),
                    LastPage: forwardRef((props, ref) => <LastPage {...props} ref={ref} />),
                    NextPage: forwardRef((props, ref) => <ChevronRight {...props} ref={ref} />),
                    PreviousPage: forwardRef((props, ref) => <ChevronLeftIcon {...props} ref={ref} />),
                    ResetSearch: forwardRef((props, ref) => <Clear {...props} ref={ref} />),
                    // Changed from FilterList
                    Search: forwardRef((props, ref) => <SearchIcon {...props} ref={ref} />),
                    SortArrow: forwardRef((props, ref) => <ArrowUpward {...props} ref={ref} />),
                    ThirdStateCheck: forwardRef((props, ref) => <Remove {...props} ref={ref} />),
                    ViewColumn: forwardRef((props, ref) => <ViewColumn {...props} ref={ref} />),
                }}
                data={nodes}
                columns={columns}
                onSearchChange={onSearchChange}
            />
        </div>
    );
}
