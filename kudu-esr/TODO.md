# TODO / FIXME

## API DESIGN

- check this for example of API:

  ```typescript
  const expireSeconds = 300
  const abiResponse = await jungle4.v1.chain.get_abi('eosio.token')
  const info = await jungle4.v1.chain.get_info()
  const header = info.getTransactionHeader(expireSeconds)
  const action = Action.from(
      {
          authorization: [
              {
                  actor: 'corecorecore',
                  permission: 'active',
              },
          ],
          account: 'eosio.token',
          name: 'transfer',
          data: {
              from: 'corecorecore',
              to: 'teamgreymass',
              quantity: '0.0042 EOS',
              memo: '',
          },
      },
      abiResponse.abi
  )
  const transaction = Transaction.from({
      ...header,
      actions: [action],
  })
  const result = await jungle4.v1.chain.compute_transaction(transaction)
  ```

- also check here: <https://github.com/greymass/eosio-signing-request-demo/blob/master/examples/resolve.js>

- look at <https://github.com/wharfkit/signing-request/blob/master/src/abi.ts>

## MISC

- report bug for wharfkit.request creation: duplicate context_free_actions, missing context_free_data
  <https://github.com/wharfkit/signing-request/blob/master/src/signing-request.ts#L410>
  see tx def: <https://docs.eosnetwork.com/docs/latest/advanced-topics/transactions-protocol/>
