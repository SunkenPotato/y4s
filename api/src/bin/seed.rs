//! `cargo run --bin seed`: migrates, truncates every table and loads demo data. Safe to re-run.

use std::collections::HashMap;

use chrono::{Datelike, Duration, NaiveDate, Utc};
use corner_api::auth::hash_password_blocking;
use corner_api::config::Config;
use corner_api::db;
use rand::seq::SliceRandom;
use rand::{rngs::StdRng, SeedableRng};
use serde_json::json;
use uuid::Uuid;

const ADMIN_EMAIL: &str = "admin@ucla.edu";
const ADMIN_PASSWORD: &str = "corner-admin-dev";
const DEMO_EMAIL: &str = "maya.r@g.ucla.edu";
const DEMO_PASSWORD: &str = "corner-dev-123";
const EXPIRED_EMAIL: &str = "old.grad@ucla.edu";

// key, name, kind, city, state, lat, lng, website, domain, domain status
type SchoolSeed = (&'static str, &'static str, &'static str, &'static str, &'static str, f64, f64, &'static str, &'static str, &'static str);
const SCHOOLS: &[SchoolSeed] = &[
    ("ucla", "UCLA", "university", "Los Angeles", "CA", 34.0689, -118.4452, "https://www.ucla.edu", "ucla.edu", "approved"),
    ("smc", "Santa Monica College", "university", "Santa Monica", "CA", 34.0168, -118.4707, "https://www.smc.edu", "smc.edu", "approved"),
    ("nyu", "NYU", "university", "New York", "NY", 40.7295, -73.9965, "https://www.nyu.edu", "nyu.edu", "approved"),
    ("ut", "UT Austin", "university", "Austin", "TX", 30.2849, -97.7341, "https://www.utexas.edu", "utexas.edu", "approved"),
    ("uhs", "University High School", "high_school", "Los Angeles", "CA", 34.0436, -118.4560, "https://www.unihigh.org", "students.lausd.net", "approved"),
    ("bhhs", "Beverly Hills High School", "high_school", "Beverly Hills", "CA", 34.0605, -118.4130, "https://bhhs.bhusd.org", "stu.bhusd.org", "pending"),
];

// key, email, first, last, birth year, school key, enrolled (YYYY-MM)
type UserSeed = (&'static str, &'static str, &'static str, &'static str, i32, &'static str, &'static str);
const USERS: &[UserSeed] = &[
    ("admin", ADMIN_EMAIL, "Corner", "Admin", 1998, "ucla", "2022-09"),
    ("maya", DEMO_EMAIL, "Maya", "Rivera", 2005, "ucla", "2024-09"),
    ("jordan", "jordan.kim@g.ucla.edu", "Jordan", "Kim", 2004, "ucla", "2023-09"),
    ("priya", "priya.s@g.ucla.edu", "Priya", "Shah", 2005, "ucla", "2024-09"),
    ("marcus", "marcus.w@g.ucla.edu", "Marcus", "Webb", 2003, "ucla", "2022-09"),
    ("sofia", "sofia.g@g.ucla.edu", "Sofia", "Gutierrez", 2006, "ucla", "2025-09"),
    ("ethan", "ethan.n@g.ucla.edu", "Ethan", "Nguyen", 2004, "ucla", "2023-09"),
    ("hannah", "hannah.l@g.ucla.edu", "Hannah", "Lee", 2005, "ucla", "2024-09"),
    ("diego", "diego.m@g.ucla.edu", "Diego", "Morales", 2002, "ucla", "2022-09"),
    ("aisha", "aisha.b@g.ucla.edu", "Aisha", "Bello", 2005, "ucla", "2024-09"),
    ("noah", "noah.p@g.ucla.edu", "Noah", "Park", 2006, "ucla", "2025-09"),
    ("chloe", "chloe.d@g.ucla.edu", "Chloe", "Dubois", 2004, "ucla", "2023-09"),
    ("sam", "sam.o@g.ucla.edu", "Sam", "Okafor", 2003, "ucla", "2022-09"),
    ("liam", "liam.f@g.ucla.edu", "Liam", "Foster", 2005, "ucla", "2024-09"),
    ("lena", "lena.t@student.smc.edu", "Lena", "Torres", 2005, "smc", "2025-01"),
    ("kai", "kai.h@student.smc.edu", "Kai", "Hoffman", 2004, "smc", "2024-09"),
    ("rosa", "rosa.v@student.smc.edu", "Rosa", "Vega", 2006, "smc", "2025-09"),
    ("jayden", "jayden.c@students.lausd.net", "Jayden", "Cruz", 2009, "uhs", "2023-08"),
    ("emily", "emily.z@students.lausd.net", "Emily", "Zhang", 2010, "uhs", "2024-08"),
    ("omar", "omar.f@students.lausd.net", "Omar", "Farouk", 2009, "uhs", "2023-08"),
    ("ben", "ben.a@nyu.edu", "Ben", "Adler", 2004, "nyu", "2024-09"),
    ("grace", "grace.y@nyu.edu", "Grace", "Yoon", 2005, "nyu", "2025-09"),
    ("tyler", "tyler.r@utexas.edu", "Tyler", "Reyes", 2004, "ut", "2023-09"),
    ("olivia", "olivia.m@utexas.edu", "Olivia", "Martin", 2005, "ut", "2024-09"),
    // Waiting on the pending stu.bhusd.org domain request.
    ("ava", "ava.s@stu.bhusd.org", "Ava", "Stein", 2009, "bhhs", "2023-08"),
    // Started in 2020, so access ended 2025-09-01: shows the `expired` state.
    ("oldgrad", EXPIRED_EMAIL, "Riley", "Grant", 1998, "ucla", "2020-09"),
];

struct PlaceSeed {
    key: &'static str,
    kind: &'static str,
    name: &'static str,
    category: &'static str,
    sub: Option<&'static str>,
    price: Option<i16>,
    address: &'static str,
    lat: f64,
    lng: f64,
    school: Option<&'static str>,
    deal: Option<&'static str>,
    hours: &'static [(&'static str, &'static str)],
    status: &'static str,
    created_by: Option<&'static str>,
}

#[allow(clippy::too_many_arguments)]
const fn biz(
    key: &'static str,
    name: &'static str,
    category: &'static str,
    sub: Option<&'static str>,
    price: Option<i16>,
    address: &'static str,
    lat: f64,
    lng: f64,
    deal: Option<&'static str>,
    hours: &'static [(&'static str, &'static str)],
) -> PlaceSeed {
    PlaceSeed { key, kind: "business", name, category, sub, price, address, lat, lng, school: None, deal, hours, status: "active", created_by: None }
}

#[allow(clippy::too_many_arguments)]
const fn facility(
    key: &'static str,
    school: &'static str,
    name: &'static str,
    category: &'static str,
    sub: Option<&'static str>,
    address: &'static str,
    lat: f64,
    lng: f64,
    hours: &'static [(&'static str, &'static str)],
) -> PlaceSeed {
    PlaceSeed { key, kind: "facility", name, category, sub, price: None, address, lat, lng, school: Some(school), deal: None, hours, status: "active", created_by: None }
}

const H_RAMEN: &[(&str, &str)] = &[("Mon – Thu", "11 am – 10 pm"), ("Fri – Sat", "11 am – midnight"), ("Sun", "noon – 9 pm")];
const H_CAFE: &[(&str, &str)] = &[("Mon – Fri", "7 am – 9 pm"), ("Sat – Sun", "8 am – 8 pm")];
const H_LATE: &[(&str, &str)] = &[("Mon – Thu", "4 pm – 2 am"), ("Fri – Sun", "noon – 2 am")];
const H_LIB: &[(&str, &str)] = &[("Mon – Thu", "7:30 am – midnight"), ("Fri", "7:30 am – 6 pm"), ("Sat – Sun", "10 am – 10 pm")];
const H_GYM: &[(&str, &str)] = &[("Mon – Fri", "6 am – 11 pm"), ("Sat – Sun", "8 am – 8 pm")];
const H_SHOP: &[(&str, &str)] = &[("Mon – Fri", "9 am – 7 pm"), ("Sat", "10 am – 5 pm")];

fn places() -> Vec<PlaceSeed> {
    let mut v = vec![
        // Westwood, near UCLA
        biz("gayley", "Gayley Noodle Bar", "food", Some("Ramen"), Some(1), "1062 Gayley Ave, Los Angeles, CA 90024", 34.0632, -118.4469, Some("15% off Mon–Thu"), H_RAMEN),
        biz("strathmore", "Strathmore Tacos", "food", Some("Mexican"), Some(1), "11005 Strathmore Dr, Los Angeles, CA 90024", 34.0693, -118.4513, Some("$2 tacos Tuesdays"), H_SHOP),
        biz("lindbrook", "Lindbrook Coffee", "coffee", Some("Cafe"), Some(1), "10945 Lindbrook Dr, Los Angeles, CA 90024", 34.0598, -118.4440, Some("Free drip refill"), H_CAFE),
        biz("copy", "Westwood Copy & Print", "services", Some("Printing"), Some(1), "1089 Westwood Blvd, Los Angeles, CA 90024", 34.0619, -118.4462, Some("10% off thesis binding"), H_SHOP),
        biz("thai", "Weyburn Thai Kitchen", "food", Some("Thai"), Some(2), "10936 Weyburn Ave, Los Angeles, CA 90024", 34.0629, -118.4452, None, H_RAMEN),
        biz("bagel", "Broxton Bagel Co.", "food", Some("Bagels"), Some(1), "1071 Broxton Ave, Los Angeles, CA 90024", 34.0626, -118.4479, None, H_CAFE),
        biz("pizza", "Kinross Slice Shop", "food", Some("Pizza"), Some(1), "10900 Kinross Ave, Los Angeles, CA 90024", 34.0611, -118.4458, Some("Slice + soda $5 with student ID"), H_LATE),
        biz("pho", "Le Conte Pho", "food", Some("Vietnamese"), Some(1), "10955 Le Conte Ave, Los Angeles, CA 90024", 34.0653, -118.4436, None, H_RAMEN),
        biz("poke", "Tiverton Poke Co.", "food", Some("Poke"), Some(2), "924 Tiverton Ave, Los Angeles, CA 90024", 34.0638, -118.4425, None, H_SHOP),
        biz("greek", "Glendon Greek Grill", "food", Some("Mediterranean"), Some(2), "1100 Glendon Ave, Los Angeles, CA 90024", 34.0616, -118.4447, Some("Free hummus with any plate"), H_RAMEN),
        biz("burger", "Midvale Burger Joint", "food", Some("Burgers"), Some(2), "1347 Westwood Blvd, Los Angeles, CA 90024", 34.0575, -118.4428, None, H_RAMEN),
        biz("curry", "Hilgard Curry House", "food", Some("Japanese curry"), Some(2), "1000 Hilgard Ave, Los Angeles, CA 90024", 34.0640, -118.4408, None, H_RAMEN),
        biz("boba", "Westwood Boba Lab", "coffee", Some("Boba"), Some(1), "1055 Broxton Ave, Los Angeles, CA 90024", 34.0622, -118.4476, Some("Buy 5 drinks, get 1 free"), H_CAFE),
        biz("owl", "Night Owl Espresso", "coffee", Some("Late-night cafe"), Some(2), "10916 Kinross Ave, Los Angeles, CA 90024", 34.0612, -118.4463, None, H_LATE),
        biz("grind", "Gayley Grind", "coffee", Some("Espresso window"), Some(1), "1000 Gayley Ave, Los Angeles, CA 90024", 34.0617, -118.4477, None, H_CAFE),
        biz("market", "Village Market & Deli", "groceries", Some("Market"), Some(2), "1020 Glendon Ave, Los Angeles, CA 90024", 34.0610, -118.4452, Some("5% off with student ID"), H_CAFE),
        biz("fresh", "Westwood Fresh Grocers", "groceries", Some("Grocery store"), Some(2), "10861 Weyburn Ave, Los Angeles, CA 90024", 34.0626, -118.4433, None, H_CAFE),
        biz("karaoke", "Broxton Karaoke Lounge", "nightlife", Some("Karaoke"), Some(2), "1099 Broxton Ave, Los Angeles, CA 90024", 34.0631, -118.4481, Some("Half-off rooms Sun–Wed"), H_LATE),
        biz("diner", "Landfair Late Night Diner", "food", Some("Diner"), Some(2), "500 Landfair Ave, Los Angeles, CA 90024", 34.0705, -118.4505, None, H_LATE),
        biz("cuts", "Westwood Cuts", "services", Some("Barber"), Some(2), "1121 Glendon Ave, Los Angeles, CA 90024", 34.0624, -118.4447, Some("$5 off student cuts weekdays"), H_SHOP),
        biz("bike", "Bruin Bike Repair", "services", Some("Bike shop"), Some(2), "10928 Le Conte Ave, Los Angeles, CA 90024", 34.0655, -118.4450, None, H_SHOP),
        biz("climb", "Wilshire Climb Gym", "gym", Some("Climbing"), Some(3), "10889 Wilshire Blvd, Los Angeles, CA 90024", 34.0583, -118.4430, Some("Student membership $49/mo"), H_GYM),
        biz("yoga", "Glenrock Yoga Studio", "gym", Some("Yoga"), Some(2), "565 Glenrock Ave, Los Angeles, CA 90024", 34.0682, -118.4535, None, H_GYM),
        biz("laundry", "Kelton Laundry Co.", "services", Some("Laundromat"), Some(1), "520 Kelton Ave, Los Angeles, CA 90024", 34.0688, -118.4528, None, H_LATE),
        biz("scoops", "Levering Scoops", "food", Some("Ice cream"), Some(1), "600 Levering Ave, Los Angeles, CA 90024", 34.0676, -118.4518, None, H_LATE),
        // UCLA campus facilities
        facility("library", "ucla", "Main Library, third floor", "study", Some("Quiet floor"), "405 Hilgard Ave, Los Angeles, CA 90095", 34.0748, -118.4415, H_LIB),
        facility("wooden", "ucla", "Wooden Rec Center", "gym", Some("Rec center"), "221 Westwood Plaza, Los Angeles, CA 90095", 34.0711, -118.4459, H_GYM),
        facility("union", "ucla", "Student Union Study Lounge", "study", Some("Group study"), "308 Westwood Plaza, Los Angeles, CA 90095", 34.0705, -118.4441, H_LIB),
        facility("reading", "ucla", "North Campus Reading Room", "study", Some("Reading room"), "405 Hilgard Ave, Los Angeles, CA 90095", 34.0745, -118.4410, H_LIB),
        facility("pool", "ucla", "Canyon Rec Pool", "gym", Some("Pool"), "200 De Neve Dr, Los Angeles, CA 90095", 34.0732, -118.4528, H_GYM),
        facility("printlab", "ucla", "Engineering Print Lab", "services", Some("Printing"), "420 Westwood Plaza, Los Angeles, CA 90095", 34.0692, -118.4429, H_LIB),
        facility("foodcourt", "ucla", "South Campus Food Court", "food", Some("Food court"), "650 Charles E Young Dr S, Los Angeles, CA 90095", 34.0680, -118.4425, H_SHOP),
        // Santa Monica College
        biz("pico", "Pico Taco Stand", "food", Some("Mexican"), Some(1), "1820 Pico Blvd, Santa Monica, CA 90405", 34.0175, -118.4725, Some("Free chips with SMC ID"), H_SHOP),
        biz("pearl", "Pearl Street Coffee", "coffee", Some("Cafe"), Some(1), "1910 Pearl St, Santa Monica, CA 90405", 34.0190, -118.4705, None, H_CAFE),
        facility("smclib", "smc", "Campus Library, quiet floor", "study", Some("Quiet floor"), "1900 Pico Blvd, Santa Monica, CA 90405", 34.0168, -118.4710, H_LIB),
        // University High School
        biz("smoothie", "Texas Ave Smoothies", "food", Some("Smoothies"), Some(1), "11820 Santa Monica Blvd, Los Angeles, CA 90025", 34.0449, -118.4555, Some("$1 off after 3 pm on school days"), H_CAFE),
        biz("barrington", "Barrington Study Cafe", "coffee", Some("Cafe"), Some(1), "1600 Barrington Ave, Los Angeles, CA 90025", 34.0450, -118.4600, None, H_CAFE),
        facility("uhslib", "uhs", "Uni High Library", "study", Some("Library"), "11800 Texas Ave, Los Angeles, CA 90025", 34.0436, -118.4560, H_SHOP),
        // NYU
        biz("dumpling", "Bleecker Dumpling House", "food", Some("Dumplings"), Some(1), "171 Bleecker St, New York, NY 10012", 40.7290, -74.0010, Some("10% off with NYU ID"), H_RAMEN),
        biz("bean", "Washington Square Bean", "coffee", Some("Cafe"), Some(2), "75 Waverly Pl, New York, NY 10011", 40.7313, -73.9975, None, H_CAFE),
        // UT Austin
        biz("tacos_ut", "Guadalupe Breakfast Tacos", "food", Some("Tex-Mex"), Some(1), "2300 Guadalupe St, Austin, TX 78705", 30.2869, -97.7420, Some("Free coffee with 2 tacos before 10"), H_CAFE),
        biz("speedway", "Speedway Study Cafe", "coffee", Some("Cafe"), Some(1), "2200 Speedway, Austin, TX 78712", 30.2865, -97.7365, None, H_CAFE),
    ];
    // Two submissions waiting in the admin queue.
    let mut sushi = biz("sushi", "Tiverton Sushi Bar", "food", Some("Sushi"), Some(2), "950 Tiverton Ave, Los Angeles, CA 90024", 34.0642, -118.4420, Some("Free miso soup with any roll"), H_RAMEN);
    sushi.status = "pending";
    sushi.created_by = Some("jordan");
    let mut games = biz("games", "Westwood Board Game Cafe", "other", Some("Board games"), Some(2), "1045 Westwood Blvd, Los Angeles, CA 90024", 34.0625, -118.4466, None, H_LATE);
    games.status = "pending";
    games.created_by = Some("aisha");
    v.push(sushi);
    v.push(games);
    v
}

// place key, author key, rating, days ago, helpful votes, used deal, body
type ReviewSeed = (&'static str, &'static str, i16, i64, usize, bool, &'static str);
const REVIEWS: &[ReviewSeed] = &[
    ("gayley", "maya", 5, 6, 22, false, "The spicy miso is worth the line. Go before six or you will wait half an hour."),
    ("gayley", "jordan", 4, 12, 9, false, "Tonkotsu broth is rich without being greasy, and the soft egg is always jammy. Took a star off because the stools are tiny and you're elbow to elbow with strangers."),
    ("gayley", "priya", 5, 20, 7, true, "Got the vegetarian shoyu with extra corn. Huge portion for the price, and the 15% Mon–Thu deal brings it under twelve bucks. My go-to after Thursday labs."),
    ("gayley", "marcus", 4, 33, 5, false, "Solid ramen, fast service. The karaage side is small but crispy. Lunch rush around 12:30 is chaos, so grab a seat at the counter if you're alone."),
    ("gayley", "sofia", 3, 41, 2, false, "Broth was great but my noodles came out a little overcooked the last two times. Still decent, just not the life-changing bowl everyone on my floor promised."),
    ("gayley", "ethan", 5, 8, 6, true, "Black garlic tonkotsu is the move. Ask for firm noodles, it makes a real difference. They honor the student deal without making it weird, just show your ID."),
    ("gayley", "hannah", 4, 25, 3, false, "Cozy spot for a rainy night. The gyoza are pan-fried properly with that lacy crust. Wish they had more seating, we waited about 20 minutes on a Friday."),
    ("gayley", "diego", 5, 55, 4, false, "Been coming here since freshman year. Consistent every single time, and the staff remembers my order. The chili oil on the table is dangerously good."),
    ("gayley", "jayden", 4, 14, 3, false, "Came here with friends after a Saturday SAT prep class. The regular bowl is big enough to share if you add rice. Way cheaper than most places in Westwood."),
    ("gayley", "lena", 4, 62, 2, false, "Worth the bus ride from Santa Monica. Spicy miso has real heat, not just red color. Parking is impossible so take the Big Blue Bus and walk up Gayley."),
    ("gayley", "noah", 2, 3, 1, false, "Waited 40 minutes on a Tuesday night and my order got mixed up. Food was fine once it came, but the hype didn't match the experience this time."),
    ("gayley", "chloe", 5, 18, 4, false, "The yuzu shio is light and bright, perfect when you don't want a heavy bowl. Small menu but everything on it is good. Ask for the extra nori, it's free."),

    ("strathmore", "marcus", 5, 5, 11, true, "Two-dollar tacos on Tuesday are real and they're not tiny. Al pastor comes off the trompo with pineapple. The line wraps around the corner but moves fast."),
    ("strathmore", "sofia", 5, 10, 8, false, "My family is from Jalisco and these are legit. The salsa verde has real tomatillo tang. Get the cabeza if they still have it after 7 pm."),
    ("strathmore", "aisha", 4, 22, 4, false, "Great carne asada, just a bit salty. The horchata is huge and not too sweet. Seating is mostly outdoor, so bring a jacket at night."),
    ("strathmore", "liam", 4, 16, 3, false, "Super close to the north dorms, which is dangerous for my wallet. The veggie tacos with grilled nopales are actually good, not an afterthought."),
    ("strathmore", "jordan", 3, 47, 1, false, "Tacos are good but the burritos are mostly rice. Stick to the tacos and the Tuesday deal and you'll leave happy."),
    ("strathmore", "omar", 5, 28, 2, true, "My friends and I walk over after school on Tuesdays. Six tacos for twelve bucks keeps me full until dinner. The guy at the register is always nice to us."),
    ("strathmore", "ethan", 4, 35, 2, false, "Fish tacos on Fridays are crispy and come with a cabbage slaw that's actually seasoned. Order at the window, food is out in under ten minutes."),
    ("strathmore", "kai", 4, 70, 1, false, "Drove over after a movie in the village. The al pastor was excellent, but the chips were stale. I'd come back just for the pastor."),

    ("lindbrook", "hannah", 5, 4, 10, true, "Best place to study south of Wilshire. Outlets along the back wall, wifi that actually works, and the free drip refill means one coffee lasts all afternoon."),
    ("lindbrook", "priya", 4, 9, 6, false, "Oat latte is smooth and they don't charge extra for alt milk before 10 am. It gets loud around 3 when the high schoolers come in, so bring headphones."),
    ("lindbrook", "chloe", 4, 19, 3, false, "Cute space with lots of light. Pastries come from a bakery in Culver City, and the almond croissant is the best one I've found near campus."),
    ("lindbrook", "noah", 3, 26, 2, false, "Coffee is good but the tables are small and wobbly. Fine for a quick reading session, not great if you need to spread out notes and a laptop."),
    ("lindbrook", "emily", 5, 12, 4, true, "I do homework here after school almost every day. The baristas never rush you out, and the refill deal works with a student ID from any school."),
    ("lindbrook", "sam", 4, 38, 2, false, "The cold brew is strong enough to get me through organic chem problem sets. Card only, which caught me off guard the first time I came in."),
    ("lindbrook", "rosa", 4, 50, 1, false, "Met my study group here before finals. Quiet in the mornings, busier after lunch. The chai is properly spiced, not syrupy."),
    ("lindbrook", "marcus", 5, 60, 2, false, "The owner roasts in small batches and will tell you all about the beans if you ask. The pour-over takes a while, but it's worth the wait."),

    ("library", "aisha", 5, 7, 12, false, "The third floor is the quiet floor and people actually respect it. Grab a desk by the north windows before 10 am, they're gone by noon during midterms."),
    ("library", "jordan", 5, 15, 7, false, "Huge tables, good lighting and outlets at every carrel. The bathrooms on this floor are cleaner than the ones downstairs. My default spot for finals."),
    ("library", "sofia", 4, 23, 4, false, "Silent means silent here. Someone shushed my highlighter once. It gets cold, so bring a sweater. Wifi drops in the far corner near the stacks."),
    ("library", "ethan", 4, 30, 3, false, "Open until midnight most weeknights and 24 hours during finals week. The group rooms on this floor need a reservation through the library site."),
    ("library", "diego", 3, 44, 2, false, "Great for focus, but finding an open seat during week 9 is basically impossible. Go early or try the overflow tables on the second floor."),
    ("library", "liam", 5, 2, 5, false, "Underrated sunset view from the west windows. I get more done here in two hours than in a whole day at my apartment."),
    ("library", "maya", 4, 52, 3, false, "My go-to for writing papers. The carrels have little shelves, so you can keep books open without taking over the whole table."),
    ("library", "noah", 4, 66, 1, false, "Reliable and quiet. Vending machines are on the ground floor only, so pack snacks. The water fountain by the elevator has a bottle filler."),

    ("wooden", "marcus", 4, 6, 8, false, "The weight room is packed from 4 to 7 pm, but mornings are empty. Squat racks are first-come, first-served and people mostly follow the 30-minute rule."),
    ("wooden", "sam", 5, 13, 6, false, "Free with registration and the basketball courts stay open late. Pickup games after 8 pm are competitive but friendly, even if you're rusty."),
    ("wooden", "priya", 4, 21, 3, false, "The group fitness classes are a steal. Wednesday morning cycling is my favorite. Locker rooms are fine, just bring your own lock."),
    ("wooden", "diego", 3, 39, 2, false, "Equipment is solid, but a few cardio machines have been broken for weeks. The climbing wall is a fun bonus though, and the staff there are great."),
    ("wooden", "noah", 4, 27, 2, false, "Good place to start lifting as a beginner. The front desk staff will walk you through the machines if you ask. Towel service costs extra."),
    ("wooden", "chloe", 5, 48, 2, false, "Dance studios can be booked for club practice, and the mirrors and sound system are great. Bring your student ID, they scan it every time."),
    ("wooden", "aisha", 4, 58, 1, false, "Fine for a quick lift between classes, and the showers are always clean. I wish the Sunday hours were longer though."),

    ("copy", "hannah", 5, 11, 7, true, "Got my senior thesis bound here with the 10% student discount. It took one day, the cover lettering came out crisp, and they caught a page order mistake for me."),
    ("copy", "ethan", 4, 24, 3, false, "Cheapest color printing in the village. The self-serve machines take cards, and the staff stay patient when you're panicking ten minutes before a deadline."),
    ("copy", "lena", 4, 36, 1, false, "Printed posters for an SMC club event here. Fair prices, and they matched the colors to my file better than the shop near my campus did."),
    ("copy", "jordan", 3, 49, 1, false, "Fine for printing, but the lunch line is long and only one person works the counter. Order online and pick up to skip the wait."),
    ("copy", "sofia", 5, 68, 2, false, "They laminated my lab reference sheets for almost nothing. The owner is super friendly and gave me tips on formatting my poster."),

    ("thai", "maya", 4, 17, 5, false, "The pad see ew is smoky with real wok char. Spice levels are honest: a 3 here is a 5 anywhere else. Lunch specials before 3 come with soup."),
    ("thai", "liam", 5, 9, 3, false, "The khao soi on the specials board is amazing when they have it. Portions are big enough that I always have leftovers for the next day."),
    ("thai", "kai", 3, 45, 1, false, "Green curry was good, but service got slow once the dinner rush hit. Ordering delivery is probably the better move here."),
    ("thai", "rosa", 4, 75, 1, false, "The Thai iced tea is perfect and the basil chicken hit the spot. The dining room is small, so go before 6 or after 8."),

    ("bagel", "ethan", 5, 3, 4, false, "Hand-rolled bagels with a real chewy crust. The everything bagel with scallion cream cheese is my 8 am fuel before lecture."),
    ("bagel", "hannah", 4, 16, 2, false, "Good bagels and fast service. The coffee is just okay, so get it somewhere else. The morning line moves quickly even when it looks long."),
    ("bagel", "emily", 4, 29, 2, false, "My mom drops me off here before school on Fridays. The egg and cheese bagel is cheap, huge and ready in about five minutes."),
    ("bagel", "diego", 3, 80, 1, false, "The bagels are solid, but they run out of the good flavors by 10 am. Weekend mornings are a zoo, so go on a weekday."),

    ("pizza", "noah", 4, 5, 3, true, "A slice plus a soda for five dollars with a student ID is unbeatable. The pepperoni cups curl up and get crispy at the edges."),
    ("pizza", "jayden", 5, 20, 2, false, "My friends and I come here after basketball. It's cheap and fast, and they're open until 1 am on weekends if we stay out late."),
    ("pizza", "sam", 3, 42, 1, false, "The cheese slice was a little floppy, but the white pizza with ricotta was great. Grab the corner seat by the window if it's open."),
    ("pizza", "aisha", 4, 57, 1, false, "A reliable late-night slice. The garlic knots are worth adding. They sometimes give you the student deal even if you forget to ask."),

    ("pho", "jordan", 5, 12, 4, false, "The pho tai is clean and fragrant, and they give you a plate piled with herbs. Exactly what I want when I'm sick during midterms."),
    ("pho", "chloe", 4, 34, 2, false, "The broth is solid, but the bun cha is even better. Quick lunch: we were in and out in about 25 minutes on a weekday."),
    ("pho", "priya", 4, 63, 1, false, "The vegetarian pho actually has flavor here, not just salty water. It's a small place, but the service is fast and friendly."),

    ("poke", "sofia", 4, 8, 3, false, "Fresh fish and generous scoops. Spicy tuna with the ponzu sauce is my combo. It's pricey, but you get what you pay for here."),
    ("poke", "marcus", 3, 31, 1, false, "Good quality, but $17 for a regular bowl adds up fast on a student budget. At least the toppings are all included."),
    ("poke", "liam", 4, 46, 1, false, "It's close to the medical center, so it's busy at noon. Order ahead on their app and walk straight past the line."),

    ("greek", "maya", 5, 22, 4, true, "The chicken shawarma plate comes with fluffy rice and a garlic sauce I would drink if allowed. Free hummus with any plate when you show a student ID."),
    ("greek", "hannah", 4, 37, 2, false, "The falafel is crispy outside and green inside, as it should be. Seating is limited, so take it to go and eat on the Janss Steps."),
    ("greek", "omar", 5, 15, 2, false, "Best gyro near my school. They don't skimp on the meat, and the fries come topped with feta and oregano."),
    ("greek", "kai", 4, 54, 1, false, "The lamb plate was great, just a bit slow at dinner. The lemon potatoes are a sleeper hit, so order a side of them."),

    ("burger", "sam", 4, 10, 3, false, "Smash burger with crispy edges on a soft potato bun, plus hand-cut fries. Expect to spend about fifteen bucks with a drink."),
    ("burger", "diego", 4, 40, 1, false, "A good burger, but it's a bit of a walk south of Wilshire. The milkshakes are thick enough that you need a spoon."),
    ("burger", "jordan", 3, 65, 1, false, "The burger was decent, but the fries were soggy by the time I got back to my dorm. Eat there, don't take it to go."),

    ("curry", "ethan", 4, 14, 3, false, "Japanese curry with a crispy katsu cutlet for under fifteen dollars. You pick the spice level from 1 to 10, and 5 is already plenty."),
    ("curry", "noah", 5, 28, 2, false, "My comfort food after a long lab. The cheese curry omurice is ridiculous in the best way, and the portion is huge."),
    ("curry", "lena", 3, 59, 1, false, "The curry was tasty, but there was way more rice than curry on the plate. Ask for extra sauce when you order."),

    ("boba", "emily", 5, 6, 5, true, "The brown sugar boba is perfectly chewy, and you can pick 25% sweetness. The punch card gets you a free drink after five."),
    ("boba", "chloe", 4, 18, 2, false, "The fruit teas taste fresh, not syrupy. The line gets long after 3 pm when school lets out, so come earlier if you can."),
    ("boba", "jayden", 4, 33, 2, false, "The taro milk tea is good, and the crema topping is worth the extra 75 cents. Staff are quick even when it's busy."),
    ("boba", "priya", 3, 71, 1, false, "The drinks are good, but the pearls were hard on my last visit. Hit or miss depending on the time of day you go."),

    ("owl", "liam", 5, 4, 4, false, "Open until 2 am on weeknights, which saved me during finals. The espresso is good, and there's a big communal table in the back."),
    ("owl", "aisha", 4, 20, 2, false, "Great late-night study vibe. The music gets a little loud after 11, so bring earplugs if you need total quiet."),
    ("owl", "marcus", 4, 43, 1, false, "Decent cold brew, and they serve real food late, not just pastries. The grilled cheese is the move at midnight."),

    ("grind", "sofia", 4, 13, 2, false, "A small walk-up window with fast espresso drinks. Perfect for grabbing a cortado on the way to a 9 am class."),
    ("grind", "sam", 3, 50, 1, false, "The coffee is okay and prices are fair, but there's no seating at all. It's strictly a grab-and-go spot."),

    ("market", "hannah", 4, 9, 3, true, "Good for quick groceries without leaving Westwood. The hot deli sandwiches are cheap and filling, and you get 5% off with a student ID."),
    ("market", "diego", 4, 30, 1, false, "My spot for late snacks and basics. The produce is a bit pricey, but the deli counter makes up for it."),
    ("market", "rosa", 3, 67, 1, false, "A small selection and kind of cramped, but convenient when you just need milk and eggs on the way home."),

    ("fresh", "maya", 4, 27, 2, false, "A wide produce selection and a decent bulk section. Prices are better than the little market by the dorms."),
    ("fresh", "chloe", 4, 53, 1, false, "The weekly sales on fruit and frozen meals help a lot on a student budget. It gets crowded on Sunday afternoons."),

    ("karaoke", "jordan", 5, 11, 4, true, "The private rooms are clean and the song list is massive, K-pop included. Half-off rooms Sunday to Wednesday makes it cheap with a group."),
    ("karaoke", "sofia", 4, 32, 2, false, "Fun for birthdays. The snacks are overpriced, but the sound system is great and the mics actually work."),
    ("karaoke", "kai", 4, 61, 1, false, "We booked a room for six after midterms. Reservations were easy online, and the staff checked in without being annoying."),

    ("diner", "liam", 4, 7, 3, false, "A classic diner with big breakfast plates served all day. The pancakes are fluffy and the coffee refills keep coming."),
    ("diner", "noah", 4, 24, 2, false, "Open late on weekends and a short walk from the dorms. The chicken and waffles are the move at 1 am."),
    ("diner", "emily", 3, 44, 1, false, "The food is fine, but service was slow when it got busy. The milkshakes are good though, especially the malted one."),

    ("cuts", "marcus", 5, 15, 3, true, "The best fade I've gotten in Westwood. Five dollars off for students on weekdays, and they take walk-ins until 6 pm."),
    ("cuts", "sam", 4, 41, 1, false, "A clean shop with good barbers. Book online if you want a Saturday slot, because walk-ins wait a long time."),

    ("bike", "aisha", 5, 19, 3, false, "They fixed my flat in fifteen minutes and didn't try to upsell me. They also sell good U-locks at a fair price."),
    ("bike", "ethan", 4, 39, 1, false, "The tune-up was thorough and quick. They'll show you how to adjust your own brakes if you ask nicely."),

    ("climb", "priya", 5, 10, 4, true, "The student membership is a great deal compared to other gyms. Lots of bouldering routes, and they reset a wall every week."),
    ("climb", "jordan", 4, 26, 2, false, "A friendly community and good beginner classes. It gets crowded after 6 pm, so weekday afternoons are best."),
    ("climb", "lena", 4, 56, 1, false, "Worth the trip from SMC for the bouldering. The showers and lockers are clean, and day passes are reasonable."),

    ("yoga", "chloe", 5, 12, 3, false, "A calm studio with great teachers. The sunrise flow on Tuesdays is my favorite way to start the week."),
    ("yoga", "hannah", 4, 47, 1, false, "Classes are a little pricey, but your first week is free and mats are provided. The heated class is intense."),

    ("laundry", "noah", 3, 21, 2, false, "The machines work and it's cheaper than my building, but half the dryers need two cycles to get anything dry."),
    ("laundry", "diego", 4, 55, 1, false, "Clean, has free wifi, and their app pings you when your load is done so you can wait at the cafe next door."),

    ("scoops", "maya", 5, 9, 3, false, "The ube and black sesame swirl is unreal. They'll let you sample three flavors before you decide, which I always need."),
    ("scoops", "omar", 4, 36, 1, false, "Good ice cream and a short walk from campus. The waffle cones are made in-house and you can smell them from the street."),

    ("union", "sam", 4, 5, 3, false, "Comfy chairs, lots of outlets, and you can talk at a normal volume. Great for group projects when the library is too strict."),
    ("union", "aisha", 3, 29, 1, false, "Convenient, but it gets loud during the lunch rush. It's much better in the mornings before 11."),
    ("union", "liam", 4, 49, 1, false, "There are microwaves nearby, so you can heat up food between classes. The couches by the windows go first."),

    ("reading", "priya", 5, 4, 5, false, "A gorgeous old reading room with high ceilings and big wooden tables. It fills up during finals, but it's worth getting there early."),
    ("reading", "maya", 4, 30, 2, false, "Silent and beautiful, but the wifi is weak in here. Download your readings before you sit down."),
    ("reading", "ethan", 4, 58, 1, false, "The best place on campus for reading physical books. Every desk has a lamp, and nobody talks above a whisper."),

    ("pool", "marcus", 4, 17, 2, false, "An outdoor pool with mountain views. Lap lanes are open in the mornings and usually not crowded before 9."),
    ("pool", "sofia", 5, 42, 1, false, "The best place to relax on a sunny afternoon. Plenty of lounge chairs and a little snack bar right next to it."),

    ("printlab", "jordan", 4, 8, 2, false, "You get a free printing quota every quarter, the printers are fast, and the staff can fix jams in seconds."),
    ("printlab", "hannah", 3, 34, 1, false, "Useful, but it closes early on Fridays. Plan ahead if you have a deadline at the end of the week."),

    ("foodcourt", "noah", 4, 6, 2, false, "Quick lunch options between science classes. The rice bowls are the best value, and the line moves fast."),
    ("foodcourt", "diego", 3, 27, 1, false, "Long lines at noon, and the menu doesn't change much from week to week. Fine in a pinch between labs."),
    ("foodcourt", "liam", 4, 51, 1, false, "A good salad bar, and they take meal swipes. The outdoor seating is nice on warm days."),

    ("thai", "chloe", 4, 4, 2, false, "Ordered the drunken noodles at medium and it had a real kick. The dining room is tiny but the food comes out fast, even on Friday nights."),
    ("pho", "hannah", 5, 21, 2, false, "The oxtail pho on weekends is rich and worth the extra few dollars. They bring you a pot of tea while you wait, which is a nice touch."),
    ("owl", "ethan", 3, 16, 1, false, "Good for night owls, but outlets are scarce and the good seats go by 9 pm. The matcha latte is surprisingly good though."),
    ("burger", "marcus", 5, 2, 3, false, "The double smash with grilled onions is the best burger near campus. Ask for the spicy aioli on the side for your fries."),
    ("climb", "sam", 4, 37, 1, false, "Auto-belay walls mean you can climb solo between classes. The yoga room upstairs is included with the student membership."),
    ("union", "maya", 4, 14, 2, false, "My spot for group projects since you can actually talk here. The whiteboards on the second floor are first-come, first-served."),
    ("pico", "lena", 5, 5, 4, true, "Cheap, fast tacos right across from campus. Free chips with an SMC ID, and the salsa bar has four kinds."),
    ("pico", "kai", 4, 20, 2, false, "The carnitas are crispy and juicy. Lines are short in the afternoon, so it's easy to grab something between classes."),
    ("pico", "rosa", 4, 40, 1, false, "My spot before evening classes. The burritos are big enough for two meals if you're on a budget."),

    ("pearl", "rosa", 5, 11, 2, false, "A quiet coffee shop with reliable wifi. The lavender latte is a little sweet but great, and they have oat milk."),
    ("pearl", "kai", 4, 35, 1, false, "Good espresso, friendly baristas and a shaded patio. It's my default before my morning chemistry class."),

    ("smclib", "lena", 4, 16, 2, false, "The quiet floor is truly quiet and there are plenty of outlets. It closes at 8 on Fridays, so plan around that."),
    ("smclib", "rosa", 4, 46, 1, false, "Good for studying between classes. The group study rooms fill up fast, so reserve one in the morning."),

    ("smoothie", "jayden", 5, 4, 3, true, "A dollar off after 3 on school days makes this my daily stop. The mango one is the best, and it's ready in two minutes."),
    ("smoothie", "emily", 4, 13, 2, false, "The smoothies are fresh and not too sweet. They also have acai bowls if you want something more filling."),
    ("smoothie", "omar", 4, 30, 1, false, "A good snack after practice. The peanut butter banana smoothie keeps me full until dinner."),

    ("barrington", "emily", 5, 9, 2, false, "A quiet place to study after school. The staff are friendly and the hot chocolate is great on cold days."),
    ("barrington", "jayden", 4, 38, 1, false, "Good snacks and free wifi. It gets busy with college students during finals, so come right after school."),

    ("uhslib", "omar", 4, 7, 2, false, "Open before and after school. It's quiet, and the librarian is great at helping with research projects."),
    ("uhslib", "emily", 4, 25, 1, false, "A good spot for group projects. The computers are a bit slow, so bring your own laptop if you have one."),

    ("dumpling", "ben", 5, 6, 1, true, "Pork and chive dumplings are perfect, and it's 10% off with an NYU ID. Ten dumplings for under ten dollars."),
    ("dumpling", "grace", 4, 18, 1, false, "Great soup dumplings and quick service. The space is tiny, so take it to go and eat in Washington Square Park."),

    ("bean", "grace", 5, 10, 1, false, "Close to campus with good pastries. The cortado is the best I've had near NYU, and the baristas are fast."),
    ("bean", "ben", 4, 33, 1, false, "Good coffee and nice for people watching, but seating is limited. Mornings before 9 are your best bet."),

    ("tacos_ut", "tyler", 5, 5, 1, true, "Breakfast tacos with free coffee before 10. The migas taco is the best one, and the salsa roja has a kick."),
    ("tacos_ut", "olivia", 4, 22, 1, false, "Cheap and fast, perfect before an 8 am class. The salsa verde is spicy, so go easy on it the first time."),

    ("speedway", "olivia", 5, 9, 1, false, "A great study spot with long tables and outlets everywhere. The cold brew is strong and refills are cheap."),
    ("speedway", "tyler", 4, 31, 1, false, "Good snacks, quiet music and fast wifi. It fills up during finals, so get there before noon."),
];

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = Config::from_env();
    let pool = db::connect(&config.database_url).await?;
    db::migrate(&pool).await?;

    for (place, author, _, _, _, _, body) in REVIEWS {
        let n = body.chars().count();
        assert!((50..=400).contains(&n), "review {place}/{author} is {n} chars");
    }

    let mut tx = pool.begin().await?;
    sqlx::query(
        "TRUNCATE review_votes, reports, reviews, places, sessions, verification_codes, school_domains, users, schools CASCADE",
    )
    .execute(&mut *tx)
    .await?;

    let now = Utc::now();
    let mut school_ids: HashMap<&str, Uuid> = HashMap::new();
    for (key, name, kind, city, state, lat, lng, website, ..) in SCHOOLS {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO schools (id, name, kind, city, state, lat, lng, website) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
            .bind(id)
            .bind(name)
            .bind(kind)
            .bind(city)
            .bind(state)
            .bind(lat)
            .bind(lng)
            .bind(website)
            .execute(&mut *tx)
            .await?;
        school_ids.insert(key, id);
    }

    // Demo users share one hash so seeding stays fast; the admin gets its own password.
    let demo_hash = hash_password_blocking(DEMO_PASSWORD);
    let admin_hash = hash_password_blocking(ADMIN_PASSWORD);
    let mut user_ids: HashMap<&str, Uuid> = HashMap::new();
    let mut user_schools: HashMap<&str, Uuid> = HashMap::new();
    for (i, (key, email, first, last, birth_year, school, enrolled)) in USERS.iter().enumerate() {
        let enrolled_on = NaiveDate::parse_from_str(&format!("{enrolled}-01"), "%Y-%m-%d")?;
        let id = Uuid::new_v4();
        let domain = corner_api::domains::domain_key(email.split_once('@').map(|(_, d)| d).unwrap_or_default());
        let (status, role) = match *key {
            "admin" => ("verified", "admin"),
            "ava" => ("pending_school", "user"),
            _ => ("verified", "user"),
        };
        let created = now - Duration::days(200 - i as i64);
        sqlx::query(
            "INSERT INTO users (id, email, email_domain, password_hash, first_name, last_name, birth_year, role, status,
                                school_id, verified_at, created_at, enrolled_on)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11, $12)",
        )
        .bind(id)
        .bind(email)
        .bind(&domain)
        .bind(if *key == "admin" { &admin_hash } else { &demo_hash })
        .bind(first)
        .bind(last)
        .bind(birth_year)
        .bind(role)
        .bind(status)
        .bind(school_ids[school])
        .bind(created)
        .bind(enrolled_on)
        .execute(&mut *tx)
        .await?;
        user_ids.insert(key, id);
        user_schools.insert(key, school_ids[school]);
    }

    for (key, .., domain, status) in SCHOOLS {
        let pending = *status == "pending";
        sqlx::query(
            "INSERT INTO school_domains (domain, school_id, status, requested_by, requested_at, decided_by, decided_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(domain)
        .bind(school_ids[key])
        .bind(status)
        .bind(pending.then(|| user_ids["ava"]))
        .bind(if pending { now - Duration::days(2) } else { now - Duration::days(365) })
        .bind((!pending).then(|| user_ids["admin"]))
        .bind((!pending).then(|| now - Duration::days(365)))
        .execute(&mut *tx)
        .await?;
    }

    let places = places();
    let mut place_ids: HashMap<&str, Uuid> = HashMap::new();
    for (i, p) in places.iter().enumerate() {
        let id = Uuid::new_v4();
        let hours = (!p.hours.is_empty()).then(|| {
            json!(p.hours.iter().map(|(d, t)| json!({ "days": d, "time": t })).collect::<Vec<_>>())
        });
        let created = if p.status == "pending" {
            now - Duration::hours(30 - i as i64 % 10)
        } else {
            now - Duration::days(180) + Duration::hours(i as i64)
        };
        sqlx::query(
            "INSERT INTO places (id, kind, name, category, subcategory, price_level, address, lat, lng, school_id,
                                 student_discount, hours, status, created_by, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
        )
        .bind(id)
        .bind(p.kind)
        .bind(p.name)
        .bind(p.category)
        .bind(p.sub)
        .bind(p.price)
        .bind(p.address)
        .bind(p.lat)
        .bind(p.lng)
        .bind(p.school.map(|s| school_ids[s]))
        .bind(p.deal)
        .bind(hours)
        .bind(p.status)
        .bind(p.created_by.map(|u| user_ids[u]))
        .bind(created)
        .execute(&mut *tx)
        .await?;
        place_ids.insert(p.key, id);
    }

    // Votes come from a deterministic shuffle of other verified students.
    let mut rng = StdRng::seed_from_u64(42);
    let voters: Vec<&str> = USERS
        .iter()
        .map(|u| u.0)
        .filter(|k| !matches!(*k, "admin" | "ava" | "oldgrad"))
        .collect();
    let mut review_ids: HashMap<(&str, &str), Uuid> = HashMap::new();
    let mut vote_total = 0;
    for (i, (place, author, rating, days, votes, used_deal, body)) in REVIEWS.iter().enumerate() {
        let id = Uuid::new_v4();
        let created = now - Duration::days(*days) - Duration::hours((i % 9) as i64 + 1);
        let visited = NaiveDate::from_ymd_opt(created.year(), created.month(), 1);
        sqlx::query(
            "INSERT INTO reviews (id, place_id, user_id, rating, body, visited_month, used_deal, created_at, updated_at,
                                  school_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8, $9)",
        )
        .bind(id)
        .bind(place_ids[place])
        .bind(user_ids[author])
        .bind(rating)
        .bind(body)
        .bind(visited)
        .bind(used_deal)
        .bind(created)
        .bind(user_schools[author])
        .execute(&mut *tx)
        .await
        .map_err(|e| anyhow::anyhow!("review {place}/{author}: {e}"))?;
        review_ids.insert((place, author), id);

        let mut pool: Vec<&str> = voters.iter().copied().filter(|v| v != author).collect();
        pool.shuffle(&mut rng);
        for voter in pool.into_iter().take(*votes) {
            sqlx::query("INSERT INTO review_votes (review_id, user_id, created_at) VALUES ($1, $2, $3)")
                .bind(id)
                .bind(user_ids[voter])
                .bind(created + Duration::hours(6))
                .execute(&mut *tx)
                .await?;
            vote_total += 1;
        }
    }

    sqlx::query(
        "INSERT INTO reports (id, reporter_id, target_type, target_id, reason, details, created_at)
         VALUES ($1, $2, 'review', $3, 'spam', $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(user_ids["priya"])
    .bind(review_ids[&("karaoke", "kai")])
    .bind("Reads like an ad for their booking site, not a real visit.")
    .bind(now - Duration::hours(5))
    .execute(&mut *tx)
    .await?;

    for (i, place) in ["strathmore", "library"].iter().enumerate() {
        sqlx::query("INSERT INTO saved_places (user_id, place_id, created_at) VALUES ($1, $2, $3)")
            .bind(user_ids["maya"])
            .bind(place_ids[place])
            .bind(now - Duration::days(3 - i as i64))
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    println!("Seeded Corner:");
    println!("  {} schools ({} approved domains, 1 pending: stu.bhusd.org)", SCHOOLS.len(), SCHOOLS.len() - 1);
    println!("  {} users, {} places (2 pending), {} reviews, {} helpful votes, 1 open report", USERS.len(), places.len(), REVIEWS.len(), vote_total);
    println!();
    println!("  Admin:     {ADMIN_EMAIL} / {ADMIN_PASSWORD}");
    println!("  Demo user: {DEMO_EMAIL} / {DEMO_PASSWORD}  (2 saved places)");
    println!("  Expired:   {EXPIRED_EMAIL} / {DEMO_PASSWORD}  (enrolled 2020-09, access ended 2025-09-01)");
    Ok(())
}
