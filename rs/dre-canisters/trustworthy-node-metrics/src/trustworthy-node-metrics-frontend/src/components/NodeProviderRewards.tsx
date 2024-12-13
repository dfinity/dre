import React, { useEffect, useState } from 'react';
import {formatDateToUTC, getLatestRewardRange, LoadingIndicator, setNodeProviderRewardsData} from '../utils/utils';
import {Box, Collapse, Grid, IconButton, Paper, TableContainer, Typography} from '@mui/material';
import { Principal } from '@dfinity/principal';
import { NodeProviderRewards } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';
import { WidgetNumber } from './Widgets';
import { boxStyleWidget } from '../Styles';
import { GridColDef, GridRowsProp } from '@mui/x-data-grid';
import Table from "@mui/material/Table";
import TableHead from "@mui/material/TableHead";
import TableRow from "@mui/material/TableRow";
import TableCell from "@mui/material/TableCell";
import TableBody from "@mui/material/TableBody";
import {KeyboardArrowDown, KeyboardArrowUp} from "@mui/icons-material";
import {Link} from "react-router-dom";

export interface NodeProviderRewardsChartProps {
    provider: string;
  }

export const NodeProviderRewardsChart: React.FC<NodeProviderRewardsChartProps> = ({ provider }) => {
    const latestRewardRange = getLatestRewardRange();
    const [latestProviderRewards, setLatestProviderRewards] = useState<NodeProviderRewards | null>(null);
    const [isLoading, setIsLoading] = useState(true);
    const [expandedRow, setExpandedRow] = useState<number | null>(null);

    useEffect(() => {
        if (provider) {
            setNodeProviderRewardsData(latestRewardRange, Principal.fromText(provider), setLatestProviderRewards, setIsLoading);
            setExpandedRow(null);
        }
    }, [provider]);

    if (isLoading) {
        return <LoadingIndicator />;
    }

    if (!latestProviderRewards) {
        return <p>No latestNodeRewards</p>;
    }

    const xdr_conversion_rate = latestProviderRewards.xdr_conversion_rate;
    const rewards_xdr_old = latestProviderRewards.rewards_xdr_old;
    if (xdr_conversion_rate.length == 0 || rewards_xdr_old.length == 0) {
        return <p>No latestNodeRewards</p>;
    }
    const distribution_date = new Date(Number(latestProviderRewards.ts_distribution) * 1000);
    const rows: GridRowsProp = latestProviderRewards.computation_log.map((data, index) => {
        return {
            id: index,
            col0: index,
            col1: data,
        };
    });
    const colDef: GridColDef[] = [
        { field: 'col0', headerName: 'Step', width: 100},
        { field: 'col1', headerName: 'Description', width: 1500},
    ];

    const computationData = latestProviderRewards.computation_data;
    const computationRows = computationData.node_provider_rewardables.map((node) => {
        const assignedMetrics = computationData.assigned_metrics.find(([principal]) => principal.toString() === node.node_id.toString());
        const status = assignedMetrics ? "assigned" : "unassigned";
        const failureRateRewardingPeriod = assignedMetrics ? computationData.failure_rate_rewarding_period.find(([principal]) => principal.toString() === node.node_id.toString())?.[1].toFixed(4) : computationData.unassigned_fr.toFixed(4);
        const failureRateActivePeriod = computationData.avg_assigned_failure_rate.find(([principal]) => principal.toString() === node.node_id.toString())?.[1].toFixed(4) || "N/A";
        const dailyIdiosyncraticFailureRate = computationData.node_daily_fr.find(([principal]) => principal.toString() === node.node_id.toString());
        const rewardsXdrNotAdjusted = Math.round((computationData.rewards_xdr_no_penalty_total.find(([principal]) => principal.toString() === node.node_id.toString())?.[1] || 0) / 10000) || "N/A";
        const rewardsXdr = Math.round((computationData.rewards_xdr.find(([principal]) => principal.toString() === node.node_id.toString())?.[1] || 0) / 10000) || "N/A";
        const rewardsMultiplier = Math.round((computationData.rewards_multiplier.find(([principal]) => principal.toString() === node.node_id.toString())?.[1] || 0) * 100) + "%" || "N/A";
        const averageFailureRate = assignedMetrics ? (assignedMetrics[1]
            .map(metric => metric.failure_rate)
            .reduce((sum, rate) => sum + rate, 0) / assignedMetrics[1].length).toFixed(4) : null;

        return {
            node_id: node.node_id.toString(),
            status,
            node_type: node.node_type,
            region: node.region,
            averageFailureRate: averageFailureRate,
            failure_rate_active_idio: failureRateActivePeriod,
            failure_rate_rewarding_period_idio: failureRateRewardingPeriod,
            rewards_multiplier: rewardsMultiplier,
            rewards_xdr_not_adjusted: rewardsXdrNotAdjusted,
            rewards_xdr: rewardsXdr,
            assigned_metrics: assignedMetrics ? assignedMetrics[1] : null,
            daily_idiosyncratic_fr: dailyIdiosyncraticFailureRate ? dailyIdiosyncraticFailureRate[1] : null,
        };
    })

    return (
        <>
            <Grid item xs={12} md={6}>
            <Box sx={boxStyleWidget('left')}>
                <WidgetNumber value={distribution_date.toDateString()} title="Date" sxPaper={{ width: '300px' }}/>
                <WidgetNumber value={(Number(latestProviderRewards.rewards_xdr_permyriad_no_reduction) / 10000).toString()} title="Expected Rewards XDR No Reduction" sxPaper={{ width: '300px' }}/>
                <WidgetNumber value={(Number(latestProviderRewards.rewards_xdr_permyriad) / 10000).toString()} title="Expected Rewards XDR"/>
                <WidgetNumber value={(Number(latestProviderRewards.xdr_conversion_rate) / 10000).toString()} title="Conversion rate"/>
            </Box>
            </Grid>
            <Grid item xs={12} md={6}>
            <Box sx={boxStyleWidget('right')}>
                <WidgetNumber value={Math.round(Number(latestProviderRewards.rewards_xdr_permyriad) / Number(latestProviderRewards.xdr_conversion_rate)).toString()} title="Expected Rewards ICP"  sxValue={{ color: '#FFCC00' }} />
                <WidgetNumber value={Math.round(Number(rewards_xdr_old[0]) / 100000000).toString()} title="Last Rewards ICP Received"  sxValue={{ color: '#FFCC00' }} />
            </Box>
            </Grid>
            <Grid item xs={12} md={12}>
                <TableContainer component={Paper}>
                    <Table aria-label="collapsible table">
                        <TableHead>
                            <TableRow>
                                <TableCell />
                                <TableCell>Node</TableCell>
                                <TableCell>Status</TableCell>
                                <TableCell>Node Type</TableCell>
                                <TableCell>Region</TableCell>
                                <TableCell>Avg. Failure Rate (active period)</TableCell>
                                <TableCell>Avg. Idiosyncratic Failure Rate (active period)</TableCell>
                                <TableCell>Avg. Idiosyncratic Failure Rate (rewarding period)</TableCell>
                                <TableCell>Rewards Multiplier</TableCell>
                                <TableCell>Rewards XDR Not Adjusted</TableCell>
                                <TableCell>Rewards XDR</TableCell>
                            </TableRow>
                        </TableHead>
                        <TableBody>
                            {computationRows
                                .sort((a, b) => a.status.localeCompare(b.status))
                                .map((row, index) => (
                                <React.Fragment key={index}>
                                    <TableRow>
                                        <TableCell>
                                            <IconButton
                                                aria-label="expand row"
                                                size="small"
                                                onClick={() => setExpandedRow(index === expandedRow ? null : index)}
                                            >
                                                {index === expandedRow ? <KeyboardArrowUp /> : row.status === "assigned" ? <KeyboardArrowDown /> : null}
                                            </IconButton>
                                        </TableCell>
                                        <TableCell>
                                            <Typography key={index} gutterBottom variant="subtitle2" sx={{ color: 'text.disabled' }} component="div">
                                                <Link to={`/nodes/${row.node_id}`} className="custom-link">
                                                    {row.node_id}
                                                </Link>
                                            </Typography>
                                        </TableCell>
                                        <TableCell>{row.status}</TableCell>
                                        <TableCell>{row.node_type}</TableCell>
                                        <TableCell>{row.region}</TableCell>
                                        <TableCell>{row.averageFailureRate}</TableCell>
                                        <TableCell>{row.failure_rate_active_idio}</TableCell>
                                        <TableCell>{row.failure_rate_rewarding_period_idio}</TableCell>
                                        <TableCell>{row.rewards_multiplier}</TableCell>
                                        <TableCell>{row.rewards_xdr_not_adjusted}</TableCell>
                                        <TableCell>{row.rewards_xdr}</TableCell>
                                    </TableRow>
                                    <TableRow>
                                        <TableCell style={{ paddingBottom: 0, paddingTop: 0 }} colSpan={9}>
                                            <Collapse in={index === expandedRow} timeout="auto" unmountOnExit>
                                                <Box sx={{ margin: 1 }}>
                                                    <Typography variant="h6" gutterBottom component="div">
                                                        Daily Metrics
                                                    </Typography>
                                                    <Table size="small" aria-label="purchases">
                                                        <TableHead>
                                                            <TableRow>
                                                                <TableCell>Date</TableCell>
                                                                <TableCell>Subnet Assigned</TableCell>
                                                                <TableCell>Blocks Proposed</TableCell>
                                                                <TableCell>Blocks Failed</TableCell>
                                                                <TableCell>Failure Rate</TableCell>
                                                                <TableCell align="right">Failure Rate (idiosyncratic)</TableCell>
                                                            </TableRow>
                                                        </TableHead>
                                                        <TableBody>
                                                            {row.assigned_metrics?.map((metric, metricIndex) => (
                                                                <TableRow key={metricIndex}>
                                                                    <TableCell>{formatDateToUTC(new Date(Number(metric.ts) / 1000000))}</TableCell>
                                                                    <TableCell>{metric.subnet_assigned.toString()}</TableCell>
                                                                    <TableCell>{metric.num_blocks_proposed.toString()}</TableCell>
                                                                    <TableCell>{metric.num_blocks_failed.toString()}</TableCell>
                                                                    <TableCell>{metric.failure_rate.toFixed(4)}</TableCell>
                                                                    <TableCell align="right">{row.daily_idiosyncratic_fr?.[metricIndex].toFixed(4)}</TableCell>
                                                                </TableRow>
                                                            ))}
                                                        </TableBody>
                                                    </Table>
                                                </Box>
                                            </Collapse>
                                        </TableCell>
                                    </TableRow>
                                </React.Fragment>
                            ))}
                        </TableBody>
                    </Table>
                </TableContainer>
            </Grid>
        </> 
    );
};

export default NodeProviderRewardsChart;
