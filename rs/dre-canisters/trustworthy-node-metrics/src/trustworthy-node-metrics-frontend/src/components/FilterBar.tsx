import React, { useState } from 'react';
import { Box, Snackbar, Alert } from '@mui/material';
import { LocalizationProvider, DatePicker, ArrowRightIcon } from '@mui/x-date-pickers';
import { AdapterDateFns } from '@mui/x-date-pickers/AdapterDateFns';
import { addMonths, isAfter, differenceInMonths } from 'date-fns';

export interface PeriodFilter {
  dateStart: Date;
  dateEnd: Date;
}

interface FilterBarProps {
  filters: PeriodFilter;
  setFilters: React.Dispatch<React.SetStateAction<PeriodFilter>>;
}

const FilterBar: React.FC<FilterBarProps> = ({ filters, setFilters }) => {
  const [error, setError] = useState<string | null>(null);

  const handleDateStartChange = (newValue: Date | null) => {
    if (newValue !== null) {
      const updatedEndDate = filters.dateEnd;
      if (updatedEndDate && isAfter(updatedEndDate, newValue) && differenceInMonths(updatedEndDate, newValue) <= 2) {
        setFilters((prev) => ({ ...prev, dateStart: newValue }));
        setError(null);
      } else {
        setError('The "To" date must be within 2 months of the "From" date.');
        setFilters((prev) => ({ ...prev, dateStart: newValue, dateEnd: addMonths(newValue, 2) }));
      }
    }
  };

  const handleDateEndChange = (newValue: Date | null) => {
    newValue?.setUTCMilliseconds(999)
    if (newValue !== null && filters.dateStart) {
      if (isAfter(newValue, filters.dateStart)) {
        if (differenceInMonths(newValue, filters.dateStart) <= 2) {
          setFilters((prev) => ({ ...prev, dateEnd: newValue }));
          setError(null);
        } else {
          setError('The "To" date must be within 2 months of the "From" date.');
          setFilters((prev) => ({ ...prev, dateEnd: addMonths(filters.dateStart, 2) }));
        }
      } else {
        setError('The "To" date must be after the "From" date.');
      }
    }
  };

  return (
    <Box mt={2}>
      <LocalizationProvider dateAdapter={AdapterDateFns}>
        <Box display="flex" alignItems="center">
          <DatePicker
            label="From"
            value={filters.dateStart}
            onChange={handleDateStartChange}
            format="dd/MM/yyy"
          />
          <ArrowRightIcon />
          <DatePicker
            label="To"
            value={filters.dateEnd}
            onChange={handleDateEndChange}
            format="dd/MM/yyy"
          />
        </Box>
      </LocalizationProvider>

      <Snackbar open={!!error} autoHideDuration={6000} onClose={() => setError(null)} anchorOrigin={{ vertical: 'top', horizontal: 'center' }}>
        <Alert onClose={() => setError(null)} severity="error" sx={{ width: '100%' }}>
          {error}
        </Alert>
      </Snackbar>
    </Box>
  );
};

export default FilterBar;
