import React from 'react';
import { Principal } from "@dfinity/principal";
import { trustworthy_node_metrics } from "../../../declarations/trustworthy-node-metrics";
import { DailyNodeMetrics, NodeRewardsArgs, NodeRewardsMultiplier, NodeProviderRewardsArgs, NodeProviderRewards } from "../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did";
import { PeriodFilter } from "../components/FilterBar";
import { Box, CircularProgress } from "@mui/material";
import { WidgetNumber } from '../components/Widgets';
import { boxStyleWidget } from '../Styles';
export interface ChartData {
  date: Date ;
  dailyNodeMetrics: DailyNodeMetrics | null;
}

export const dateToNanoseconds = (date: Date): bigint => {
  const millisecondsSinceEpoch = date.getTime();
  const nanosecondsSinceEpoch = BigInt(millisecondsSinceEpoch) * BigInt(1_000_000);
  return nanosecondsSinceEpoch;
};

export const generateChartData = (periodFilter: PeriodFilter, dailyData: DailyNodeMetrics[]): ChartData[] => {
    const { dateStart, dateEnd } = periodFilter;
    const chartData: ChartData[] = [];

    const currentDate = new Date(dateStart);

    while (currentDate <= dateEnd) {
      const dailyDataEntry = dailyData.find(data => {
        const date = new Date(Number(data.ts) / 1000000);
        
        return date.getUTCFullYear() === currentDate.getUTCFullYear() &&
        date.getUTCMonth() === currentDate.getUTCMonth() &&
        date.getUTCDate() === currentDate.getUTCDate()
      });
      
      chartData.push({
          date: new Date(currentDate),
          dailyNodeMetrics: dailyDataEntry ? dailyDataEntry : null,
      });

      currentDate.setDate(currentDate.getDate() + 1);
    }

    return chartData;
};


export const getFormattedDates = (period: PeriodFilter): string[] => {
    const { dateStart, dateEnd } = period;
    const dates: string[] = [];
    const currentDate = new Date(dateStart);
  
    while (currentDate <= dateEnd) {
      dates.push(
        currentDate.toLocaleDateString('UTC', { month: 'short', day: 'numeric' }).replace(" ", "\n")
      );
      currentDate.setDate(currentDate.getDate() + 1);
    }
  
    return dates;
  };

export const formatDateToUTC = (date: Date): string => {
    const day = String(date.getUTCDate()).padStart(2, '0');
    const month = String(date.getUTCMonth() + 1).padStart(2, '0'); // Months are zero-indexed
    const year = date.getUTCFullYear();
  
    return `${day}-${month}-${year}`;
  };
  
export const computeAverageFailureRate = (data: number[]): number => {
    if (data.length === 0) return 0;
    const sum = data.reduce((acc, val) => acc + val, 0);
    return sum / data.length;
  };

export const dateStartToEnd = (date: Date): Date => {
  return new Date(
    Date.UTC(
      date.getUTCFullYear(),
      date.getUTCMonth(),
      date.getUTCDate(),
      23, 59, 59, 999
    )
  )
};
export const getDateRange = () => {
  const now = new Date();
  const dateStart = new Date(
    Date.UTC(
      now.getUTCFullYear(),
      now.getUTCMonth() - 1,
      1, 0, 0, 0, 0 
    )
  );

  const dateEnd = new Date(
    Date.UTC(
      now.getUTCFullYear(),
      now.getUTCMonth(),
      now.getUTCDate(),
      23, 59, 59, 999
    )
  );

  return { dateStart, dateEnd };
};

export const getLatestRewardRange = () => {
  const now = new Date();
  const currentDay = now.getUTCDate();

  const dateEnd = new Date(
    Date.UTC(
      now.getUTCFullYear(),
      currentDay <= 13 ? now.getUTCMonth() - 1 : now.getUTCMonth(),
      13, 23, 59, 59, 999
    )
  );

  const dateStart = new Date(
    Date.UTC(
      dateEnd.getUTCFullYear(),
      dateEnd.getUTCMonth() - 1 ,
      14, 0, 0, 0, 0 
    )
  );
  return { dateStart, dateEnd };
};

export const setNodeRewardsData = async (
  periodFilter: PeriodFilter, 
  node_id: Principal,
  setNodeRewards: React.Dispatch<React.SetStateAction<NodeRewardsMultiplier | null>>,
  setIsLoading: React.Dispatch<React.SetStateAction<boolean>>) => {
  try {
    setIsLoading(true);

    const request: NodeRewardsArgs = {
      from_ts: dateToNanoseconds(periodFilter.dateStart),
      to_ts: dateToNanoseconds(periodFilter.dateEnd),
      node_id: node_id,
    };
    const nodeRewardsResponse = await trustworthy_node_metrics.node_rewards(request);

    setNodeRewards(nodeRewardsResponse);
  } catch (error) {
    console.error("Error fetching node:", error);
  } finally {
    setIsLoading(false);
  }
};

export const setNodeProviderRewardsData = async (
  periodFilter: PeriodFilter, 
  provider: Principal,
  setProviderRewards: React.Dispatch<React.SetStateAction<NodeProviderRewards | null>>,
  setIsLoading: React.Dispatch<React.SetStateAction<boolean>>) => {
  try {
      setIsLoading(true);
  
      const request: NodeProviderRewardsArgs = {
        from_ts: dateToNanoseconds(periodFilter.dateStart),
        to_ts: dateToNanoseconds(periodFilter.dateEnd),
        node_provider_id: Principal.from(provider),
      };
      const nodeRewardsResponse = await trustworthy_node_metrics.node_provider_rewards(request);
  
      setProviderRewards(nodeRewardsResponse);
    } catch (error) {
      console.error("Error fetching node:", error);
    } finally {
      setIsLoading(false);
  }
};

export const LoadingIndicator: React.FC = () => (
  <Box
    sx={{
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'center',
      height: '100vh',
    }}
  >
    <CircularProgress />
  </Box>
);

export const NodeMetricsStats: React.FC<{ stats: NodeRewardsMultiplier['rewards_multiplier_stats'] | null }> = ({ stats }) => (
  <Box sx={boxStyleWidget('left')}>
      <WidgetNumber value={stats ? stats.days_assigned.toString() : "0"} title="Days Assigned" />
      <WidgetNumber value={stats ? stats.days_unassigned.toString() : "0"} title="Days Unassigned" />
      <WidgetNumber value={stats ? stats.blocks_proposed.toString() : "0"} title="Blocks Proposed Total" />
      <WidgetNumber value={stats ? stats.blocks_failed.toString() : "0"} title="Blocks Failed Total" />
  </Box>
);

export const NodePerformanceStats: React.FC<{ rewardMultiplier: string , baseRewardsXDR: string}> = ({ rewardMultiplier, baseRewardsXDR }) => (
  <Box sx={boxStyleWidget('right')}>
      <WidgetNumber value={rewardMultiplier} title="Reward Multiplier" sxValue={{ color: '#FFCC00' }} />
      <WidgetNumber value={baseRewardsXDR} title="Base Monthly Rewards XDR" sxValue={{ color: '#FFCC00' }} />
  </Box>
);


