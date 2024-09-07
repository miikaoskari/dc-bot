const { SlashCommandBuilder } = require('discord.js');
const { exec } = require('child_process');
const fs = require('fs');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('video')
        .setDescription('send link to video and send content to channel'),
    async execute(interaction) {
        const videoUrl = interaction.options.getString('url');
        const outputFilePath = 'downloaded_video.mp4';

        // download video from link (shell execute yt-dlp)
        exec(`yt-dlp -o ${outputFilePath} ${videoUrl}`, async (error, stdout, stderr) => {
            if (error) {
                console.error(`Error downloading video: ${error.message}`);
                await interaction.reply('There was an error downloading the video.');
                return;
            }

            if (stderr) {
                console.error(`yt-dlp stderr: ${stderr}`);
            }

            console.log(`yt-dlp stdout: ${stdout}`);

            // send video to channel
            await interaction.reply({
                content: `Here is your video from ${videoUrl}`,
                files: [outputFilePath]
            });

            // Clean up the downloaded file
            fs.unlinkSync(outputFilePath);
        });
    },
};