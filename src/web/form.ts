async function submit() {
	let body: Login[] | BodyInit = [
		new Login(
			(<HTMLInputElement>document.getElementById('name')).value,
			(<HTMLInputElement>document.getElementById('username')).value,
			(<HTMLInputElement>document.getElementById('password')).value
		),
	];

	body = JSON.stringify(body);

	let options: RequestInit = {
		method: 'POST',
		body: body,
		headers: [['Content-Type', 'application/json']],
	};

	let response = await fetch('/api/v1/new', options);

	if (response.ok) {
		window.location.href = '/query';
		if (window.localStorage.getItem('show-add-success') == null) {
			window.localStorage.setItem(
				'show-add-success',
				String(
					window.confirm(
						'Successfully added a login. Would you like to show this message in the future?'
					)
				)
			);
		} else if (window.localStorage.getItem('show-add-success') == 'true') {
			window.alert('Successfully added a login.');
		}

		return;
	}

	console.log(response.status);
	console.log(response.statusText);
	window.alert('Failed to add a login.');
}

class Login {
	name: string;
	username: string;
	password: string;

	constructor(name: string, username: string, password: string) {
		this.name = name;
		this.username = username;
		this.password = password;
	}
}
