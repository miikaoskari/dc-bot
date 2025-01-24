const { SlashCommandBuilder } = require('discord.js');
const { exec } = require('child_process');
const fs = require('fs');

const isValidUrl = (url) => {
    try {
        new URL(url);
    } catch (e) {
        return false;
    }
    const redditRegex = /^https?:\/\/(www\.)?reddit\.com\/.+$/;
    const tiktokRegex = /^https?:\/\/(www\.)?tiktok\.com\/.+$/;
    const tiktokVmRegex = /^https?:\/\/(vm\.)?tiktok\.com\/.+$/;
    const youtubeRegex = /^https?:\/\/(www\.)?youtube\.com\/.+$/;
    const youtubeShortRegex = /^https?:\/\/youtu\.be\/.+$/;
    const xRegex = /^https?:\/\/(www\.)?x\.com\/.+$/;
    const instagramRegex = /^https?:\/\/(www\.)?instagram\.com\/.+$/;
    return redditRegex.test(url) || tiktokRegex.test(url) || tiktokVmRegex.test(url) || youtubeRegex.test(url) || youtubeShortRegex.test(url) || xRegex.test(url) || instagramRegex.test(url);
};

module.exports = {
    data: new SlashCommandBuilder()
        .setName('video')
        .setDescription('send link to video and send content to channel')
        .addStringOption(option => 
            option.setName('url')
                .setDescription('url of the video to download')
                .setRequired(true)),
    async execute(interaction) {
        const videoUrl = interaction.options.getString('url');
        const outputFilePath = 'downloaded_video.mp4';

        // Validate the URL
        if (!isValidUrl(videoUrl)) {
            await interaction.reply('only tiktok, reddit, youtube, x and instagram links are supported');
            return;
        }

        // Defer the reply to give more time for processing
        await interaction.deferReply();

        // download video from link (shell execute yt-dlp)
        // use --max-filesize to prevent downloads of over 50M
        // discord allows max of 10M add 40M of approximation buffer
        // produces *.part files that need to be cleaned up
        exec(`yt-dlp -o ${outputFilePath} ${videoUrl} --max-filesize 50M`, async (error, stdout, stderr) => {
            try {
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
                await interaction.followUp({
                    content: `here is your video`,
                    files: [outputFilePath]
                });

                // Clean up the downloaded file
                fs.unlinkSync(outputFilePath);

                // Delete all *.part files
                fs.readdirSync('.').forEach(file => {
                    if (file.endsWith('.part')) {
                        fs.unlinkSync(file);
                    }
                });
            } catch (err) {
                console.error(`Unexpected error: ${err.message}`);
                await interaction.followUp('An unexpected error occurred.');
                if (fs.existsSync(outputFilePath)) {
                    fs.unlinkSync(outputFilePath);
                }
                // Delete all *.part files
                fs.readdirSync('.').forEach(file => {
                    if (file.endsWith('.part')) {
                        fs.unlinkSync(file);
                    }
                });
            }
        });
    },
};
