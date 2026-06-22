// expect: total=10 code=5 comment=3 blank=2 shebang=0
// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

// simple counter contract
contract Counter {
	uint public count;
	function increment() public { count += 1; }
}
