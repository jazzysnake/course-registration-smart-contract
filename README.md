# Course registration smart contract

_This is a smart contract that handles the creation of couses by teachers,
and the student's registrations to the courses. Originally an assignement in
BUTE's blockchain course, but I created it in !ink as practice for [PMC hackathon](https://metaversechampionship.gg/)_

The contract handles:

- Creation of courses by teachers
- Students registration to courses
- Swapping of courses between students

Courses have:

- An "owner" Teacher
- Start dates
- Max capacities

Courses can only be swapped before their starting dates
