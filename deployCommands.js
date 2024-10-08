const { REST, Routes } = require("discord.js");
const { clientId, token } = require("/app/config.json");
const fs = require("node:fs");
const path = require("node:path");

const commands = [];
const foldersPath = path.join(__dirname, "commands");
const commandFolders = fs.readdirSync(foldersPath);

for (const folder of commandFolders) {
  const commandsPath = path.join(foldersPath, folder);
  const commandFiles = fs
    .readdirSync(commandsPath)
    .filter((file) => file.endsWith(".js"));
  for (const file of commandFiles) {
    const filePath = path.join(commandsPath, file);
    const command = require(filePath);
    if ("data" in command && "execute" in command) {
      commands.push(command.data.toJSON());
    } else {
      console.log(
        `[WARNING] The command at ${filePath} is missing a required "data" or "execute" property.`
      );
    }
  }
}

const rest = new REST({ version: "10" }).setToken(token);

(async () => {
  try {
    console.log("Started deleting all application (/) commands.");

    // Fetch all existing commands and delete them
    const currentCommands = await rest.get(
      Routes.applicationCommands(clientId)
    );

    const deletePromises = currentCommands.map((cmd) =>
      rest.delete(`${Routes.applicationCommands(clientId)}/${cmd.id}`)
    );

    await Promise.all(deletePromises);

    console.log("Successfully deleted all outdated application (/) commands.");

    console.log(
      `Started refreshing ${commands.length} application (/) commands.`
    );

    // Register new commands
    const data = await rest.put(Routes.applicationCommands(clientId), {
      body: commands,
    });

    console.log(
      `Successfully reloaded ${data.length} application (/) commands.`
    );
  } catch (error) {
    console.error(error);
  }
})();
