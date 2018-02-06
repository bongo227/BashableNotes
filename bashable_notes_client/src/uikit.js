import React from "react";

export const Container = ({children}) => (
	<div className="uk-container">{children}</div>
);

export const Spinner = ({}) => (
	<div uk-spinner=""></div>
);