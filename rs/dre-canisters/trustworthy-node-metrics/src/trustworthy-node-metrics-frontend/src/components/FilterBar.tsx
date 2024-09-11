import React from 'react';
import { Toolbar, Box, Divider } from '@mui/material';
import { LocalizationProvider, DatePicker, ArrowRightIcon } from '@mui/x-date-pickers';
import { AdapterDateFns } from '@mui/x-date-pickers/AdapterDateFns';

export interface PeriodFilter {
  dateStart: Date;
  dateEnd: Date;
}

interface FilterBarProps {
  filters: PeriodFilter;
  setFilters: React.Dispatch<React.SetStateAction<PeriodFilter>>;
}

const FilterBar: React.FC<FilterBarProps> = ({ filters, setFilters }) => {
  const handleMonthChange = (newMonth: Date | null) => {
    if (newMonth) {
      const month = newMonth.getUTCMonth();
      const year = newMonth.getUTCFullYear();
  
      const beginRewardPeriod = new Date(Date.UTC(year, month - 1, 14, 0, 0, 0, 0));
      const endRewardPeriod = new Date(Date.UTC(year, month, 13, 23, 59, 59, 999));
  
      setFilters(() => ({
        dateStart: beginRewardPeriod,
        dateEnd: endRewardPeriod,
      }));
    } else {
      setFilters(prev => ({
        dateStart: prev.dateStart,
        dateEnd: prev.dateEnd,
      }));
    }
  };

  const handleDateStartChange = (newValue: Date | null) => {
    if (newValue !== null) {
      setFilters((prev) => ({ ...prev, dateStart: newValue }));
    }
  };

  const handleDateEndChange = (newValue: Date | null) => {
    if (newValue !== null) {
      setFilters((prev) => ({ ...prev, dateEnd: newValue }));
    }
  };

  return (
    <Toolbar>
      <LocalizationProvider dateAdapter={AdapterDateFns}>
        <Box display="flex" alignItems="center">
          <DatePicker label="Reward Period" views={['month']} value={filters.dateEnd} onChange={handleMonthChange} />
          <Divider orientation="vertical" flexItem sx={{ mx: 2 }} />
          <DatePicker
            label="From"
            value={filters.dateStart}
            onChange={handleDateStartChange}
          />
          <ArrowRightIcon />
          <DatePicker
            label="To"
            value={filters.dateEnd}
            onChange={handleDateEndChange}
          />
        </Box>
      </LocalizationProvider>
    </Toolbar>
  );
};

export default FilterBar;
