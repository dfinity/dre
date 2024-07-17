import { FC } from "react"
import { Grid, TextField } from "@mui/material"
import * as React from "react"


type SearchCompProps = {
    setSearchTerm: Function,
}

interface SearchState {
    typingTimeout: any
}

export const SearchComp: FC<SearchCompProps> = ({ setSearchTerm }) => {
    const [state, setState] = React.useState<SearchState>({
        typingTimeout: 0
    });
    const changeName = (event) => {
        if (state.typingTimeout) {
           clearTimeout(state.typingTimeout);
        }
    
        setState({
           typingTimeout: setTimeout(function () {
            setSearchTerm(event.target.value);
             }, 350)
        });
    }
    return (
        <Grid container spacing={1} maxHeight={90} minHeight={90} padding={2}>
            <TextField id="outlined-basic" label="node or subnet id" variant="outlined" sx={{
                width: 750,
            }} onChange={changeName}/>
        </Grid>
    )
}