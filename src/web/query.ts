// TODO: https://alistapart.com/article/neveruseawarning/
async function remove_login(id: string) {
	if (!window.confirm('Are you sure you want to remove this login?')) {
		return;
	}

	let url: URL = new URL('/api/v1/remove', window.location.origin);
	url.searchParams.append('id', id);

	const res = await fetch(url, {
		method: 'DELETE',
	});

	if (res.ok) {
		document.getElementById(id)!.remove();
		if (window.localStorage.getItem('show-remove-success') == null) {
			window.localStorage.setItem(
				'show-remove-success',
				String(
					window.confirm(
						'Successfully removed a login. Would you like to show these messages in the future?'
					)
				)
			);
		} else if (
			window.localStorage.getItem('show-remove-success') == 'true'
		) {
			window.alert('Successfully removed a login.');
		}

		return;
	}

	console.error(res.status);
	console.error(res.url);
	window.alert('Failed to delete the login');
}
