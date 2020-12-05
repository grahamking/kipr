**kip - Keeps Internet Passwords** Password manager. Command line script to keep usernames and passwords in gnupg encrypted text files.

If you live on the command line, this is a simple, fast, secure, and evergreen way to manage your passwords. I use it multiple times a day.

`kip get|add|list|edit|del [filepart] [--username USERNAME] [--notes NOTES] [--prompt] [--print]`

# INSTALL

- Linux x86_64: Download the pre-built binary from **Releases** in the sidebar.
- Rust programmers: Clone the repo and `cargo build`.
- Everyone else: Eh, I dunno. Open a ticket and say what OS you need.

Make sure you have a gnupg key pair:
[GnuPG HOWTO](https://help.ubuntu.com/community/GnuPrivacyGuardHowto).

# COMMANDS

## add

`kip add example.com --usename username`

What it does:

 1. Generates a random password
 2. Writes username and password to text file `~/.kip/passwords/example.com`
 3. Encrypts and signs it by running `gpg --encrypt --sign --armor`
 4. Copies the new password to your clipboard

Add optional notes: `kip add example.com --username username --notes "My notes"`.
You can ask to be pompted for the password, instead of using a random one: `kip add example.com --username username --prompt`

## get

`kip example.com`

What it does:

 1. Looks for `~/.kip/passwords/*example.com*`, decrypts it by running `gpg --decrypt`
 2. Prints your username in bold, and any notes your stored.
 3. Copies your password to the clipboard

## list

`kip list "*.org"`

List contents of your password directory. [filepart] argument is a glob to filter the directory list. You can use ls too!

## edit

`kip edit example.com --username newuser`

Change the username inside a password file.  [filepart] is the file to edit, and --username sets a new username.

## del

`kip del example.com`

Delete a password file. [filepart] is the file to delete. You can use rm too!

# DEPENDENCIES

gnupg to encrypt password files, xclip (linux) or pbcopy (OSX) to copy password to clipboard.

On Ubuntu / Debian: `sudo apt-get install gnupg xclip`

# CONFIGURATION

If you want to use different commands to encrypt / decrypt your files, want longer passwords, etc, you can. Copy this snippet (the built-in default config) to `~/.kip/kip.conf`, and customise it. It's an INI file, using = or : as the delimiter. Make sure the `home` path does not end with a slash.

```
[gnupg]
key_fingerprint:
encrypt_cmd:gpg --quiet --encrypt --sign --default-recipient-self --armor
decrypt_cmd:gpg --quiet --decrypt
[passwords]
home:~/.kip/passwords
len:19
[tools]
clip: # empty means 'pbcopy' on OSX, 'xclip' elsewhere
```

# NOTES

You should really _really_ backup your `~/.kip/passwords/` directory. I use [Borg Backup](https://www.borgbackup.org/).

[GnuPG](http://www.gnupg.org/) is secure, open, multi-platform, and will probably be around forever. Can you say the same thing about the way you store your passwords currently?

I was using the excellent [Keepass](http://en.wikipedia.org/wiki/KeePass) when I got concerned about it no longer being developed or supported. How would I get my passwords out? So I wrote this very simple wrapper for gnupg.

If you live in the command line, I think you will find **kip** makes your life a little bit better.

If you put it in a sync-ed service (Dropbox, Google Drive, etc) you can have it on multiple computers. The files are encrypted, the service won't be able to spy on you.

`kipr` is a Rust port of [kip](https://github.com/grahamking/kip) (Python).

# FILES

There's 0 magic involved. Your accounts details are in text files, in your home directory. Each one is encrypted with your public key and signed with your private key. You can ditch **kip** at any time.

Browse your files: `ls ~/.kip/passwords/`

Display contents manually: `gpg -d ~/.kip/passwords/facebook`
