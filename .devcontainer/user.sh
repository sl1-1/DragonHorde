NEW_GID=1865600513
NEW_UID=1865601103
REMOTE_USER=vscode
sed -n "s/${REMOTE_USER}:[^:]*:\([^:]*\):\([^:]*\):[^:]*:\([^:]*\).*/OLD_UID=\1;OLD_GID=\2;HOME_FOLDER=\3/p" /etc/passwd
sed -n "s/\([^:]*\):[^:]*:${NEW_UID}:.*/EXISTING_USER=\1/p" /etc/passwd
sed -n "s/\([^:]*\):[^:]*:${NEW_GID}:.*/EXISTING_GROUP=\1/p" /etc/group
if [ -z "$OLD_UID" ]; then echo "Remote user not found in /etc/passwd ($REMOTE_USER).";
elif [ "$OLD_UID" = "$NEW_UID" -a "$OLD_GID" = "$NEW_GID" ]; then echo "UIDs and GIDs are the same ($NEW_UID:$NEW_GID).";
elif [ "$OLD_UID" != "$NEW_UID" -a -n "$EXISTING_USER" ]; then echo "User with UID exists ($EXISTING_USER=$NEW_UID).";
else if [ "$OLD_GID" != "$NEW_GID" -a -n "$EXISTING_GROUP" ]; then echo "Group with GID exists ($EXISTING_GROUP=$NEW_GID).";
NEW_GID="$OLD_GID";
fi;
echo "Updating UID:GID from $OLD_UID:$OLD_GID to $NEW_UID:$NEW_GID.";
sed -i -e "s/\(${REMOTE_USER}:[^:]*:\)[^:]*:[^:]*/\1${NEW_UID}:${NEW_GID}/" /etc/passwd;
if [ "$OLD_GID" != "$NEW_GID" ]; then sed -i -e "s/\([^:]*:[^:]*:\)${OLD_GID}:/\1${NEW_GID}:/" /etc/group; fi;
chown -R $NEW_UID:$NEW_GID $HOME_FOLDER; fi;
