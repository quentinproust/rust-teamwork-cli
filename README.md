# Get your company id
To find your company id, you can look at the url when you are working on teamwork. For exemple, if your url
is something like https://my-awesome-company.eu.teamwork.com/ then your company id is my-awesome-company.

More informations on Teamwork Developer Website

# Get your token
You can find your token by clicking on your profile in [teamwork](https://altima1.eu.teamwork.com/) in the upper right corner, then modify details, then select the integrations tab and click on show your token

# Init
```
git clone https://github.com/quentinproust/rust-teamwork-cli.git
cd rust-teamwork-cli
cargo build
cargo run -- auth -c $companyID -t $your_token 
```

Be careful when adding time if you had any vacations, moreover the automatic last filled date will take the last filled date +1 and could result in a wrong day
# Add time
```
cargo run -- interactive
```

for example to fulfill 13 days of work considering each day is 8 work hours and the start date being 2019-06-24 
```
start date 2019-06-24 
Hours 104
Dry run at no
```

You can check if the program ran successfully by going to your profile in [teamwork](https://altima1.eu.teamwork.com/) in the upper right corner, then see profile, then time

