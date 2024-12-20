import React, { useEffect, useMemo, useState } from 'react';
import { Box, Grid, Paper, Typography } from '@mui/material';
import Divider from '@mui/material/Divider';
import { useParams } from 'react-router-dom';
import { paperStyle } from '../Styles';
import {
    SubnetFailureRate, SubnetNodeMetrics
} from "../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did";
import InfoFormatter from "./NodeInfo";
import {
    BarPlot,
    ChartsAxisHighlight,
    ChartsTooltip,
    ChartsXAxis,
    ChartsYAxis,
    LinePlot,
    MarkPlot,
    ResponsiveChartContainer
} from "@mui/x-charts";
import {LoadingIndicator} from "../utils/utils";
import {trustworthy_node_metrics} from "../../../declarations/trustworthy-node-metrics";
import {Principal} from "@dfinity/principal";

export interface SubnetPageProps {
    subnetsData: SubnetFailureRate[]
}

export const SubnetPage: React.FC<SubnetPageProps> = ({ subnetsData }) => {
    const { subnet } = useParams();
    const [subnetNodeMetrics, setSubnetNodeMetrics] = useState<SubnetNodeMetrics[] | null>(null);
    const [isLoading, setIsLoading] = useState(true);

    useEffect(() => {
        if (subnet) {
            const updateSubnetNodeMetrics = async () => {
                try {
                    setIsLoading(true);
                    const metrics = await trustworthy_node_metrics.subnet_nodes_metrics(Principal.from(subnet));

                    setSubnetNodeMetrics(metrics);
                } catch (error) {
                    console.error("Error fetching node rewards:", error);
                } finally {
                    setIsLoading(false);
                }
            };

            updateSubnetNodeMetrics();
        }
    }, [subnet]);

    if (isLoading) {
        return <LoadingIndicator />;
    }

    if (!subnetNodeMetrics) {
        return <p>No metrics for the time period selected</p>;
    }

    if (!subnet) {
        return <p>No Node provider</p>;
    }

    const failureRates = subnetsData.filter(map => map.subnet_id.toText() == subnet);
    if (!failureRates) {
        return <p>No Subnet</p>;
    }

    const xAxisConfig = [
        {
            id: 'x-axis-id',
            scaleType: 'band' as const,
            dataKey: 'date',
            categoryGapRatio: 0.4,
            barGapRatio: 0.3,
            valueFormatter: (value: Date) => {
                const oneDayBefore = new Date(value);
                oneDayBefore.setDate(value.getDate() - 1); // Adjust the date by subtracting 1 day
                return oneDayBefore
                    .toLocaleDateString('UTC', { month: 'short', day: 'numeric' })
                    .replace(" ", "\n");
            }
        }
    ];

    const yAxisConfig = [
        {
            id: 'y-axis-id',
            valueFormatter: (value: number) => `${value}`,
        }
    ];

    const seriesConfig = [
        {
            dataKey: 'systematicFailureRates',
            label: 'Systematic Failure Rate',
            color: '#FFFFFF', // Tomato Red
            type: 'bar' as const,
            valueFormatter: (value: number | null) => value ? `${value}` : '0',
        },
        {
            dataKey: 'nodeFailureRates',
            label: 'Node Failure Rate',
            color: '#FF6347', // White
            type: 'bar' as const,
            valueFormatter: (value: number | null) => value ? `${value}` : '0',
        }
    ];
    const systematicFailureMap = new Map(
        failureRates.map(entry => [
            new Date(Number(entry.ts) / 1000000).toDateString(),
            Math.round(entry.failure_rate * 10000) / 10000
        ])
    );

    return (
        <Box sx={{ p: 3 }}>
            <Paper sx={paperStyle}>
                <Grid container spacing={3}>
                    <Grid item xs={12} md={12}>
                        <Typography gutterBottom variant="h5" component="div">
                            {"Subnet"}
                        </Typography>
                        <Divider/>
                    </Grid>
                    <Grid item xs={12}>
                        <InfoFormatter name={"Subnet ID"} value={subnet ? subnet : "Anonym"} />
                    </Grid>
                    <Grid item xs={12}>
                        <Typography variant="h6" component="div" >
                            Systematic/Idiosyncratic Failure Rate
                        </Typography>
                        <Divider/>
                    </Grid>
                    {subnetNodeMetrics.map((subnetNodeMetrics, nodeIndex) => {
                        // Merge systematic and node-specific metrics
                        const dataset = subnetNodeMetrics.daily_node_metrics.map((entry) => {
                            const entryDate = new Date(Number(entry.ts) / 1000000);
                            return {
                                date: entryDate,
                                systematicFailureRates: systematicFailureMap.get(entryDate.toDateString()) ?? null,
                                nodeFailureRates: Math.round(entry.failure_rate * 10000) / 10000
                            };
                        });

                        return (
                            <Grid key={nodeIndex} item xs={12}>
                                <Typography variant="subtitle1" component="div">
                                    Node: {subnetNodeMetrics.node_id.toText()}
                                </Typography>
                                <ResponsiveChartContainer
                                    xAxis={xAxisConfig}
                                    yAxis={yAxisConfig}
                                    series={seriesConfig}
                                    dataset={dataset}
                                    height={300}
                                >
                                    <BarPlot borderRadius={9} />
                                    <ChartsAxisHighlight />
                                    <ChartsTooltip />
                                    <ChartsXAxis position="bottom" axisId="x-axis-id" />
                                    <ChartsYAxis position="left" axisId="y-axis-id" />
                                </ResponsiveChartContainer>
                            </Grid>
                        );
                    })}
                </Grid>
            </Paper>
        </Box>
    );
};