import React from 'react';
import Typography from '@mui/material/Typography';

interface InfoFormatterProps {
  name: string;
  value: string;
}

const InfoFormatter: React.FC<InfoFormatterProps> = ({ name, value }) => {
  return (
    <div>
      <Typography gutterBottom variant="subtitle1" component="div">
        {name}
      </Typography>
      <Typography gutterBottom variant="subtitle2" sx={{ color: 'text.disabled' }} component="div">
        {value}
      </Typography>
    </div>
  );
};

export default InfoFormatter;
