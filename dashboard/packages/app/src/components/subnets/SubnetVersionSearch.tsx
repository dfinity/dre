import React, { forwardRef } from 'react';
import { Link, Tooltip, Typography, makeStyles } from '@material-ui/core';
import { Table, TableColumn } from '@backstage/core-components';
import { fetchMissingGuests, fetchNodes, fetchNodesHealths, fetchSubnetVersions, fetchSubnets } from './fetch';
import { NodeHealth, Node, Subnet, Operator, SubnetUpdate } from './types';
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
import { SubnetUpdateStateIcon, stateDescription } from './subnetUpdate';

function timeSince(since: number) {
    var seconds = Math.floor((new Date()).getTime() / 1000 - since);
    var interval
    interval = Math.floor(seconds / 86400);
    if (interval >= 1) {
        return interval + "d " + Math.floor((seconds % 86400) / 3600) + "h";
    }
    interval = Math.floor(seconds / 3600);
    if (interval >= 1) {
        return interval + "h " + Math.floor((seconds % 3600) / 60) + "m";
    }
    interval = Math.floor(seconds / 60);
    if (interval >= 1) {
        return interval + "m " + (seconds % 60) + "s";
    }
    return Math.floor(seconds) + "s";
}

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

export default function SubnetVersionSearch() {
    const classes = useStyles();
    const columns: TableColumn<SubnetUpdate>[] = [
        {
            title: 'Subnet ID',
            field: 'subnet_id',
            type: 'string',
            emptyValue: '',
        },
        {
            title: 'Subnet Name',
            field: 'subnet_name',
            type: 'string',
            emptyValue: '',
        },
        {
            title: 'State',
            field: 'state',
            type: 'string',
            emptyValue: '',
            render: (rowData) => <Tooltip interactive title={
                <div style={{ maxWidth: 150 }}>
                    <Typography display="block" variant="caption">{`${rowData.state[0].toUpperCase()}${rowData.state.substring(1)}`}</Typography>
                    <Typography display="block" variant="caption" style={{ fontStyle: "italic", fontSize: "0.6rem" }} >
                        {stateDescription(rowData.state)}
                    </Typography>
                </div>
            } placement="left">
                <div>
                    <Typography display="block" variant="caption">{`${rowData.state[0].toUpperCase()}${rowData.state.substring(1)}`}</Typography>
                    <SubnetUpdateStateIcon state={rowData.state} />
                </div>
            </Tooltip>
        },
        {
            title: 'Latest proposal',
            field: 'proposal.info.id',
            type: 'string',
            render: (rowData) => <Link target="_blank" href={rowData.proposal ? `https://dashboard.internetcomputer.org/proposal/${rowData.proposal?.info.id}` : ''}>
                {rowData.proposal?.info.id ?? ""}
            </Link>
        },
        {
            title: 'Updated',
            field: 'proposal.info.proposal_timestamp_seconds',
            type: 'string',
            emptyValue: '',
            render: (rowData) => rowData.proposal ? timeSince(rowData.proposal.info?.proposal_timestamp_seconds) + " ago" : "Unknown",
        },
        {
            title: 'Patches available',
            field: 'patches_available',
            type: 'string',
            emptyValue: '',
            render: (rowData) => rowData.patches_available.length,
            searchable: false,
        },
        {
            title: 'Release Name',
            field: 'replica_release.name',
            type: 'string',
            emptyValue: '',
        },
    ];


    const subnetsVersions = fetchSubnetVersions();

    return (
        <div className={classes.containerExpanded}>
            <Table
                options={{
                    padding: 'dense',
                    emptyRowsWhenPaging: false,
                    pageSize: 100,
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
                        searchPlaceholder: 'Search for subnets by name, principal, version, state, etc. ...',
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
                data={subnetsVersions}
                columns={columns}
            />
        </div>
    );
}
