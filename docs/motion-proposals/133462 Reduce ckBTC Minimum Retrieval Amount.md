The ckBTC minimum retrieval variable is currently set to 0.001 BTC as shown by the function ‘retrieve_btc_min_amount’and. This determines the minimum ckBTC amount that can be burned and, correspondingly, the minimum native BTC amount that can be withdrawn from the minter.  
  
However, at current BTC prices this puts the minimum amount of native BTC one can withdraw from the IC between $60-$70 USD, despite there being no minimum deposit amount. We find this minimum amount to be high and contend this creates a barrier to ckBTC onboarding and adoption from both native BTC holders and IC Dapps.  
  
We propose that this amount be lowered to 0.0005 BTC ($30-$35 USD at current value) in order to facilitate the onboarding to ckBTC by guaranteeing a lower offboarding requirement and improving the ckBTC user experience. 

Whilst we recognise the potential risk this lower minimum could lead to a potentially dropped BTC withdrawal in the case of a small ckBTC unminting coupled with a large BTC fee rise,  we contend this risk is minor and is outweighed by the benefits of an improved ckBTC user experience and lower barrier to usage. 

We have put forward this proposal as a motion proposal in order to gauge community sentiment as to this move. Should this motion proposal pass, we will put forward a subsequent proposal including the code changes required.  
  
Please find more information and community discussion surrounding this topic on the forum here:  
[https://forum.dfinity.org/t/for-the-icp-community-how-to-get-the-number-of-bitcoin-confirmations-from-the-ckbtc-minter/35973](https://forum.dfinity.org/t/for-the-icp-community-how-to-get-the-number-of-bitcoin-confirmations-from-the-ckbtc-minter/35973)