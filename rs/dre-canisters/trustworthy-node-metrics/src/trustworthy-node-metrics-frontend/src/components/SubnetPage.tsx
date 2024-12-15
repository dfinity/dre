import React, { useEffect, useMemo, useState } from 'react';
import { Box, Grid, Paper, Typography } from '@mui/material';
import Divider from '@mui/material/Divider';
import { useParams } from 'react-router-dom';
import { paperStyle } from '../Styles';
import {SubnetFailureRate} from "../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did";
import InfoFormatter from "./NodeInfo";
import PerformanceChart from "./PerformanceChart";
import {
    BarPlot,
    ChartsAxisHighlight,
    ChartsTooltip,
    ChartsXAxis, ChartsYAxis,
    LinePlot,
    MarkPlot,
    ResponsiveChartContainer
} from "@mui/x-charts";

export interface SubnetPageProps {
    subnetsData: SubnetFailureRate[]
  }

export const SubnetPage: React.FC<SubnetPageProps> = ({ subnetsData }) => {
    const { subnet } = useParams();

    if (!subnet) {
        return <p>No Node provider</p>;
    }

    const failureRates = subnetsData.filter(map => map.subnet_id.toText() == subnet);
    if (!failureRates) {
        return <p>No Subnet</p>;
    }

    useEffect(() => {
        window.scrollTo(0, 0);
    }, [subnet]);

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
            dataKey: 'failureRates',
            label: 'Failed Blocks',
            color: '#FF6347',
            stack: 'total',
            type: 'bar' as const,
            valueFormatter: (value: number | null) => value ? `${value}` : '0',
        }
    ];

    const dataset = failureRates.map((entry) => ({
        date: new Date(Number(entry.ts) / 1000000),
        failureRates: entry.failure_rate
    }));

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
                        Systematic Failure Rate
                    </Typography>
                    <Divider/>
                    <ResponsiveChartContainer
                        xAxis={xAxisConfig}
                        yAxis={yAxisConfig}
                        series={seriesConfig}
                        dataset={dataset}
                        height={300}
                    >
                        <BarPlot borderRadius={9} />
                        <LinePlot />
                        <MarkPlot />
                        <ChartsAxisHighlight />
                        <ChartsTooltip />
                        <ChartsXAxis position="bottom" axisId="x-axis-id" />
                        <ChartsYAxis position="left" axisId="y-axis-id" />
                    </ResponsiveChartContainer>
                </Grid>
            </Grid>
        </Paper>
    </Box>
    );
};
