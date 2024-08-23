import React from 'react';
import Typography from '@mui/material/Typography';

interface NodeInfoProps {
  nodeId: string;
  nodeProviderId: string;
}

const NodeInfo: React.FC<NodeInfoProps> = ({ nodeId, nodeProviderId }) => {
  return (
    <div>
      <Typography gutterBottom variant="subtitle1" component="div">
        {"Node ID"}
      </Typography>
      <Typography gutterBottom variant="subtitle2" sx={{ color: 'text.disabled' }} component="div">
        {nodeId}
      </Typography>

      <Typography gutterBottom variant="subtitle1" component="div">
        {"Node Provider ID"}
      </Typography>
      <Typography gutterBottom variant="subtitle2" sx={{ color: 'text.disabled' }} component="div">
        {nodeProviderId}
      </Typography>
    </div>
  );
};

export default NodeInfo;
