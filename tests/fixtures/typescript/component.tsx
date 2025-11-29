// expect: total=11 code=5 comment=2 blank=4 shebang=0
// tsx component fixture

import React from "react";

type Props = { name: string };

export function Hello(props: Props) {
	return <div>Hello {props.name}</div>;
}
