"use strict";

class Socket {
	constructor() {
		this.socketPath = "";
	}
}

class Settings {
	/**
	 * Read the settings.
	 * @returns {{type: "sqlite"} | {type: string, socketPath: string}} The settings.
	 */
	static readSettings() {
		return JSON.parse("{}");
	}

	static connect() {
		let settings;
		try {
			settings = this.readSettings();
		} catch (e) {
			settings = { type: "sqlite" };
		}
		return settings.socketPath;
		//              ^ d: jsdoc/settings.js:12
	}

	static either(cold) {
		let held;
		if (cold) {
			held = this.readSettings();
		} else {
			held = new Socket();
		}
		return held.socketPath;
		//          ^ d: jsdoc/settings.js:5
	}
}

module.exports = { Socket, Settings };
