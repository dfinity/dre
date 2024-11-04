export const idlFactory = ({ IDL }) => {
  return IDL.Service({ 'increase_int' : IDL.Func([], [], []) });
};
export const init = ({ IDL }) => { return []; };
