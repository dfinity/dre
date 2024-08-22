import React from 'react';
import { Toolbar, Box } from '@mui/material';
import { LocalizationProvider, DatePicker } from '@mui/x-date-pickers';
import { AdapterDateFns } from '@mui/x-date-pickers/AdapterDateFns';
import ArrowRightIcon from '@mui/icons-material/ArrowRight';

export interface PeriodFilter {
  dateStart: Date;
  dateEnd: Date;
}

interface FilterBarProps {
  filters: PeriodFilter;
  setFilters: React.Dispatch<React.SetStateAction<PeriodFilter>>;
}

const FilterBar: React.FC<FilterBarProps> = ({ filters, setFilters }) => {
  const handleDateStartChange = (newValue: Date | null) => {
    setFilters(prev => ({
      ...prev,
      dateStart: newValue ?? prev.dateStart,
    }));
  };

  const handleDateEndChange = (newValue: Date | null) => {
    setFilters(prev => ({
      ...prev,
      dateEnd: newValue ?? prev.dateEnd,
    }));
  };

  return (
    <Toolbar>
      <LocalizationProvider dateAdapter={AdapterDateFns}>
        <Box display="flex" alignItems="center">
          <DatePicker
            label="From"
            value={filters.dateStart}
            onChange={handleDateStartChange}
            sx={{ mr: 2 }}
          />
          <ArrowRightIcon />
          <DatePicker
            label="To"
            value={filters.dateEnd}
            onChange={handleDateEndChange}
            sx={{ ml: 2 }}
          />
        </Box>
      </LocalizationProvider>
    </Toolbar>
  );
};

export default FilterBar;
