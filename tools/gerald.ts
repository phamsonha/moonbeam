import { init, importAccount, web3, gerald, contractAddress } from "./init-web3";

const main = async () => {
  init("http://127.0.0.1:9933")
  // console.log (web3)
  const chainId = await web3.eth.net.getId();
  console.log(`Using chain id: ${chainId}\n`);
  const nonce = await web3.eth.getTransactionCount(gerald.address);
  console.log(`nonde: ${nonce}`)
  
  // Step 1: Creating the contract.
  
  console.log(`Gerald account: ${gerald.address} (nonce: ${nonce})`);

  const contractAdd = contractAddress(gerald.address, 0);
  const code = await web3.eth.getCode(contractAdd);
  const storageAdd = await web3.eth.getBalance(contractAdd, "0");
  console.log(`Gerald contract[0]: ${contractAdd} (code: ${code.length} bytes)`);
  console.log(`           storage: ${storageAdd}`);
};

main().catch((err) => {
  console.log("Error", err);
});
