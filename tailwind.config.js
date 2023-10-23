/** @type {import('tailwindcss').Config} */
module.exports = {
	content: ['./src/web/*.{html, ts}', './dist/*.js'],
	theme: {
		extend: {},
	},
	plugins: [
		require('@tailwindcss/forms')({
			strategy: 'class',
		}),
	],
};
