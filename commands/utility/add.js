const { SlashCommandBuilder } = require('discord.js');

module.exports = {
	data: new SlashCommandBuilder()
		.setName('add')
		.setDescription('add value to user'),
	async execute(interaction) {
		await interaction.reply('value added');
	},
};
