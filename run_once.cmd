@echo off

echo '[MySQL Dump] run once begin'
rust_mysqldump.exe -t
echo [MySQL Dump] run once done

echo
echo Press any key to exit...
pause > nul
